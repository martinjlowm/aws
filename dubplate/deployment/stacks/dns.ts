import { type App, DelegatedZone } from '@martinjlowm/aws-constructs';
import { organizationIdentifiersSchema, parse } from '@martinjlowm/aws-organization';
import { Stack } from 'aws-cdk-lib';
import type { Certificate } from 'aws-cdk-lib/aws-certificatemanager';
import type { PublicHostedZone } from 'aws-cdk-lib/aws-route53';

const org = parse(process.env.AWS_ORG, organizationIdentifiersSchema);

/** Where the application is served. */
export const DOMAIN = 'dubplate.martinjlowm.dk';

/**
 * The subdomain, and the certificate for it.
 *
 * us-east-1 because CloudFront reads certificates from there and nowhere else,
 * which is the only reason this is a second stack rather than part of the site.
 * The zone comes along because ACM validates by writing a record into it, and a
 * certificate whose validation record is a region away is a deployment that
 * waits for a human.
 *
 * The zone is here rather than in the domains account so that this application
 * owns every name under it: adding a hostname is a deployment of this project,
 * not a change to the zone that carries the mail. Nothing in aws/domains has to
 * be deployed first, because the role the delegation assumes trusts the whole
 * organization.
 */
export class Dns extends Stack {
  readonly zone: PublicHostedZone;
  readonly certificate: Certificate;

  constructor(app: App, id: string) {
    super(app, `${id}-dns`, {
      // us-east-1 is CloudFront's certificate region. Everything else this
      // project deploys is in eu-west-1.
      env: { region: 'us-east-1', account: org.accounts.dubplateWeb },
      crossRegionReferences: true,
    });

    const delegated = new DelegatedZone(this, 'dubplate', {
      domainName: DOMAIN,
      domainsAccountId: org.accounts.domains,
      certificate: true,
    });

    this.zone = delegated.zone;
    // `certificate: true` above is what makes this defined.
    this.certificate = delegated.certificate!;
  }
}
