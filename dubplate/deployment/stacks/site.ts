import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import type { App } from '@martinjlowm/aws-constructs';
import { organizationIdentifiersSchema, parse } from '@martinjlowm/aws-organization';
import { Duration, RemovalPolicy, Stack } from 'aws-cdk-lib';
import type { ICertificate } from 'aws-cdk-lib/aws-certificatemanager';
import {
  AllowedMethods,
  CachePolicy,
  Distribution,
  HeadersFrameOption,
  HeadersReferrerPolicy,
  PriceClass,
  ResponseHeadersPolicy,
  ViewerProtocolPolicy,
} from 'aws-cdk-lib/aws-cloudfront';
import { S3BucketOrigin } from 'aws-cdk-lib/aws-cloudfront-origins';
import { AaaaRecord, ARecord, type IHostedZone, RecordTarget } from 'aws-cdk-lib/aws-route53';
import { CloudFrontTarget } from 'aws-cdk-lib/aws-route53-targets';
import { BlockPublicAccess, Bucket, BucketEncryption } from 'aws-cdk-lib/aws-s3';
import { BucketDeployment, CacheControl, Source } from 'aws-cdk-lib/aws-s3-deployment';

const org = parse(process.env.AWS_ORG, organizationIdentifiersSchema);

/** Where `just build-app` leaves the bundle Trunk produced. */
const BUNDLE = join(import.meta.dir, '..', '..', 'app', 'dist');

/**
 * The content security policy's allowance for the script Trunk inlines.
 *
 * Trunk writes the wasm bootstrap into index.html as an inline module rather than
 * a file, so a policy without `'unsafe-inline'` blocks it and the page renders
 * nothing at all. The choice is between allowing every inline script forever and
 * naming this one, and naming it is what a hash is for.
 *
 * Read out of the built bundle rather than written down, because the hash covers
 * the script's exact bytes and those carry the content-hashed file names Trunk
 * generated. A hardcoded value would be correct until the next build.
 *
 * Whitespace is not trimmed: the browser hashes what sits between the tags.
 */
function inlineScriptHashes(): string[] {
  const html = readFileSync(join(BUNDLE, 'index.html'), 'utf8');
  return [...html.matchAll(/<script\b[^>]*>([\s\S]*?)<\/script>/g)]
    .map(([, body]) => body)
    .filter((body) => body.trim().length > 0)
    .map((body) => `'sha256-${createHash('sha256').update(body, 'utf8').digest('base64')}'`);
}

import { DOMAIN } from './dns';

interface SiteProps {
  region: string;
  /** The delegated zone this application answers for, from the DNS stack. */
  zone: IHostedZone;
  /** The certificate for it, which CloudFront reads from us-east-1. */
  certificate: ICertificate;
}

/**
 * A bucket nobody can read and a distribution that can.
 *
 * There is no server here and no request that reaches one. A visitor gets the
 * page, two wasm modules and the fonts; every byte of audio they pick stays in
 * the tab. That is the reason this stack has no compute in it and the reason it
 * is worth saying so: an upload endpoint would be the natural shape of this
 * application and would put somebody's music library on somebody else's disk.
 */
export class Site extends Stack {
  constructor(app: App, id: string, props: SiteProps) {
    super(app, `${id}-site`, {
      env: { region: props.region, account: org.accounts.dubplateWeb },
      // The zone and the certificate are declared in us-east-1 and read here.
      crossRegionReferences: true,
    });

    const bucket = new Bucket(this, 'bucket', {
      bucketName: `${this.account}-${this.region}-dubplate`,
      encryption: BucketEncryption.S3_MANAGED,
      blockPublicAccess: BlockPublicAccess.BLOCK_ALL,
      enforceSSL: true,
      // The bundle is a build artefact and the sources are in git, so there is
      // nothing here that a rebuild cannot produce again.
      removalPolicy: RemovalPolicy.DESTROY,
      autoDeleteObjects: true,
    });

    // The analysis is one long synchronous run per track, so it goes in a
    // worker. Workers alone need none of this; `SharedArrayBuffer`, which is
    // what a threaded wasm build would want, needs cross-origin isolation. It is
    // not set, because setting it would also break every `<img>` and font that
    // is not served with CORP, and nothing here is threaded yet.
    const headers = new ResponseHeadersPolicy(this, 'headers', {
      responseHeadersPolicyName: `${id}-headers`,
      securityHeadersBehavior: {
        contentTypeOptions: { override: true },
        frameOptions: { frameOption: HeadersFrameOption.DENY, override: true },
        referrerPolicy: {
          referrerPolicy: HeadersReferrerPolicy.NO_REFERRER,
          override: true,
        },
        strictTransportSecurity: {
          accessControlMaxAge: Duration.days(365),
          includeSubdomains: true,
          override: true,
        },
        contentSecurityPolicy: {
          // `connect-src 'self'` is the whole network surface: the tool reads
          // files and talks to nothing, and this is where that claim is enforced
          // rather than asserted.
          contentSecurityPolicy: [
            "default-src 'none'",
            // 'wasm-unsafe-eval' is what a browser calls instantiating a wasm
            // module; without it the analysis does not start. The hashes are
            // Trunk's inline bootstrap, named rather than blanket-allowed.
            `script-src 'self' 'wasm-unsafe-eval' ${inlineScriptHashes().join(' ')}`,
            "worker-src 'self' blob:",
            "style-src 'self' 'unsafe-inline'",
            "font-src 'self'",
            "img-src 'self' data: blob:",
            "connect-src 'self'",
            "base-uri 'none'",
            "form-action 'none'",
            "frame-ancestors 'none'",
          ].join('; '),
          override: true,
        },
      },
    });

    const distribution = new Distribution(this, 'distribution', {
      domainNames: [DOMAIN],
      certificate: props.certificate,
      defaultRootObject: 'index.html',
      // A single-page application: a reload on a path the bucket has no object
      // for is the page, not a 404.
      errorResponses: [
        { httpStatus: 403, responseHttpStatus: 200, responsePagePath: '/index.html' },
        { httpStatus: 404, responseHttpStatus: 200, responsePagePath: '/index.html' },
      ],
      defaultBehavior: {
        origin: S3BucketOrigin.withOriginAccessControl(bucket),
        viewerProtocolPolicy: ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        allowedMethods: AllowedMethods.ALLOW_GET_HEAD,
        cachePolicy: CachePolicy.CACHING_OPTIMIZED,
        responseHeadersPolicy: headers,
        compress: true,
      },
      // Nobody outside Europe has asked for this. PRICE_CLASS_100 is the
      // difference between a distribution that costs nothing and one that does.
      priceClass: PriceClass.PRICE_CLASS_100,
    });

    // Two deployments over one bundle, because the two halves want opposite
    // cache headers.
    //
    // Trunk hashes what it compiles, so the wasm, the stylesheet and their glue
    // can be held for a year: a new build is a new name. The rest is copied
    // verbatim and keeps its name across builds, and a year-cached worker.js is
    // a visitor running last month's analysis against this month's page. Those
    // go in the second deployment and are invalidated on every deploy.
    //
    // The analysis is the reason this list is not just index.html. Trunk names a
    // worker after its binary rather than hashing it, which is what lets the page
    // reach it at a fixed path, and it is also what makes a year's cache serve
    // last month's measurements against this month's interface.
    //
    // The fonts are the exception in the other direction: copied verbatim, never
    // changed, and a new face would arrive under a new name anyway.
    const VERBATIM = [
      'index.html',
      'favicon.svg',
      'analysis.js',
      'analysis_bg.wasm',
      'analysis_loader.js',
    ];

    new BucketDeployment(this, 'hashed', {
      sources: [Source.asset(BUNDLE, { exclude: VERBATIM })],
      destinationBucket: bucket,
      distribution,
      prune: false,
      cacheControl: [CacheControl.maxAge(Duration.days(365)), CacheControl.immutable()],
    });

    // Both families, because a visitor on a v6-only mobile network is a visitor
    // who otherwise gets nothing. An alias rather than a CNAME: this is the apex
    // of its own zone, where a CNAME is not allowed to sit beside the NS records
    // that delegate it.
    const target = RecordTarget.fromAlias(new CloudFrontTarget(distribution));
    new ARecord(this, 'alias', { zone: props.zone, target });
    new AaaaRecord(this, 'alias-v6', { zone: props.zone, target });

    new BucketDeployment(this, 'verbatim', {
      sources: [Source.asset(BUNDLE, { exclude: ['*', ...VERBATIM.map((path) => `!${path}`)] })],
      destinationBucket: bucket,
      distribution,
      distributionPaths: VERBATIM.map((path) => `/${path}`),
      prune: false,
      cacheControl: [CacheControl.noCache()],
    });
  }
}
