import { type App, ZONE_DELEGATION_ROLE } from '@martinjlowm/aws-constructs';
import { organizationIdentifiersSchema, parse } from '@martinjlowm/aws-organization';
import { Duration, Stack } from 'aws-cdk-lib';
import { OrganizationPrincipal, Role } from 'aws-cdk-lib/aws-iam';
import { MxRecord, PublicHostedZone, TxtRecord } from 'aws-cdk-lib/aws-route53';

const org = parse(process.env.AWS_ORG, organizationIdentifiersSchema);

/**
 * Google Workspace for the primary domain.
 *
 * Five records, which is both what Cloudflare serves today and what the Admin
 * console recommends for this domain. Not the single `smtp.google.com` below:
 * Google asks for that on the alias domain and for these here, and the console
 * is the authority on which domain gets which.
 */
const GOOGLE_MAIL_PRIMARY = [
  { priority: 1, hostName: 'aspmx.l.google.com' },
  { priority: 5, hostName: 'alt1.aspmx.l.google.com' },
  { priority: 5, hostName: 'alt2.aspmx.l.google.com' },
  { priority: 10, hostName: 'aspmx2.googlemail.com' },
  { priority: 10, hostName: 'aspmx3.googlemail.com' },
];

/**
 * Google Workspace for the alias domain.
 *
 * One record. Google's simplified MX, which the Admin console asks for on this
 * domain and reports the five above as `Incorrect` for. Cloudflare is serving the
 * five, so this is the one record in either zone that is a change rather than a
 * transcription.
 */
const GOOGLE_MAIL_ALIAS = [{ priority: 1, hostName: 'smtp.google.com' }];

/**
 * Who may send as these domains.
 *
 * The value the Admin console gives, which is missing from both zones today:
 * mail leaves Google with nothing asserting it was allowed to. `~all` is a soft
 * fail, so a message from somewhere else is marked rather than rejected, which is
 * what to publish before knowing every sender.
 */
const GOOGLE_SPF = 'v=spf1 include:_spf.google.com ~all';

/**
 * The public halves of the keys Google signs each domain's mail with.
 *
 * Public on purpose. The private halves never leave Google, and these are
 * published so that anyone receiving a signed message can fetch one and check
 * the signature. A key here is what makes a forged message detectable rather
 * than merely unwelcome.
 *
 * One key per domain, and the two are not interchangeable. Google generates them
 * separately and signs each domain's mail with its own, so publishing the primary
 * key under the alias would fail every signature it was asked to verify.
 *
 * The selector is `google` for both, which is the name in the host below and the
 * name Google writes into the signature header so a verifier knows which record
 * to read. Each value is 410 characters, longer than the 255 a single DNS string
 * holds; `TxtRecord` splits them and a resolver joins the pieces back.
 */
const GOOGLE_DKIM_PRIMARY =
  'v=DKIM1; k=rsa; p=MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAxj1JoAdTnAZDiC279aRG0md4KM/n4WnMhr+uK/d6VmeUK+vz0Vlw1JVaHvEbs069gFwraOdotntkhjMgPGoNIMjdfyEVy6VVjPhchOpW3RVZ5m4axnqpgZqd8cYBssEwwR0hQoyXM5YaqJruvZ97kWlQMpznrawqCaIBNFUGgh+XoXKAZyKJ/4h2gmdd6OoVV/MLc4f11X/S+rWBOn4ZYaa52Ch/WmIn0rGqieI/oqNvdoAh9IuarunOw12IYWWAVKGXvLSMlMgrlDvpPGgSbVi1znJXq5e5pQ0CR13lqbEGmNuxGXK1X42hoQDWAR9D9hJDHUyj0gWxPM1nk2QyvQIDAQAB';

const GOOGLE_DKIM_ALIAS =
  'v=DKIM1; k=rsa; p=MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAzz+smkyx9jpF5SVgM7lpQNjNmTS4fBAdl9qIkN110x89SMmrKpirgpadYoWnrC6dldZ2DKcRvlLkGl9KGwlYwnPZGA5bus/YU4fXqfQdwGhFCi6X2GBMdgrV7di6i4sztjqCJEnc10cDn+CixaE6fbPLpzgeSn9xV6zf+v8x5rguKOZ/I3gR8hQRQVHpnZY+R+iQqgkOPImbmxnKmGphHJDa37bknAeS0xicEES6tX9RsFYwvCosLLNCJsVxkVUvZi+uOwIwfMh5rSx/zF0m4GijP0NRXNt+hfVbq+FFsUQ9Q+OyJMpfpWPyVicLH6eAUZfNcmIBQRwhU9qOG64taQIDAQAB';

interface ZonesProps {
  region: string;
}

/**
 * Every domain, and the door applications come in through.
 *
 * Only records belonging to a domain itself live here, which today is mail and
 * verification. Everything an application serves is a delegated subdomain in
 * that application's own account, so deploying one cannot touch the MX records
 * and deleting one cannot take them with it.
 */
export class Zones extends Stack {
  constructor(app: App, id: string, props: ZonesProps) {
    super(app, `${id}-zones`, {
      env: { region: props.region, account: org.accounts.domains },
    });

    // One role for every domain and every application.
    //
    // Trusted by the organization rather than by named accounts, which is what
    // makes adding an application a deployment of that application rather than
    // a deployment of this stack first. That trust is broad on purpose and is
    // safe because of what it can do rather than who holds it: `grantDelegation`
    // permits UPSERT and DELETE of NS records and nothing else, so an account
    // that assumes this can delegate a subdomain and cannot touch the MX
    // records, the verification TXT, or any other record here.
    //
    // What it does not prevent is one application delegating a name another was
    // going to use. That is a collision between two things this organization
    // already trusts, and the fix if it ever matters is `delegatedZoneNames` on
    // the grant, which pins each account to the names it owns.
    const delegation = new Role(this, 'delegation', {
      roleName: ZONE_DELEGATION_ROLE,
      assumedBy: new OrganizationPrincipal(org.organization),
      description: 'Write a subdomain NS record into one of these zones',
    });

    for (const domain of [martinjlowm(this), martinlowm(this)]) {
      domain.grantDelegation(delegation);
    }
  }
}

/**
 * The primary domain.
 *
 * No apex and no `www`: what Cloudflare served for both was its own proxy
 * addresses in front of a Heroku application that no longer runs. Reproducing
 * addresses that mean nothing off Cloudflare would be a zone that answers and a
 * site that does not.
 */
function martinjlowm(scope: Stack): PublicHostedZone {
  const zone = new PublicHostedZone(scope, 'martinjlowm', {
    zoneName: 'martinjlowm.dk',
    comment: 'Declared in aws/domains. Records added by hand will be removed.',
  });

  new MxRecord(scope, 'martinjlowm-mail', {
    zone,
    ttl: Duration.minutes(5),
    values: GOOGLE_MAIL_PRIMARY,
  });

  new TxtRecord(scope, 'martinjlowm-spf', {
    zone,
    ttl: Duration.minutes(5),
    values: [GOOGLE_SPF],
  });

  new TxtRecord(scope, 'martinjlowm-dkim', {
    zone,
    recordName: 'google._domainkey',
    ttl: Duration.minutes(5),
    values: [GOOGLE_DKIM_PRIMARY],
  });

  return zone;
}

/**
 * The second domain, which delivers to the same mailbox.
 *
 * Apex and `www` are dropped here for the same reason as above.
 */
function martinlowm(scope: Stack): PublicHostedZone {
  const zone = new PublicHostedZone(scope, 'martinlowm', {
    zoneName: 'martinlowm.dk',
    comment: 'Declared in aws/domains. Records added by hand will be removed.',
  });

  new MxRecord(scope, 'martinlowm-mail', {
    zone,
    ttl: Duration.minutes(5),
    values: GOOGLE_MAIL_ALIAS,
  });

  // One record set, two strings. Route 53 holds one set per name and type, so the
  // verification and the policy cannot be two constructs: declaring them
  // separately would be two resources fighting over the same apex TXT.
  //
  // The verification string is how Google proves it owns this domain. Removing
  // it is Google treating the domain as unverified.
  new TxtRecord(scope, 'martinlowm-txt', {
    zone,
    ttl: Duration.minutes(5),
    values: ['google-site-verification=nTDBdrWiI8w1eD0XyhNxJ7PoIl1c3VRJbq4uIgPEHWQ', GOOGLE_SPF],
  });

  new TxtRecord(scope, 'martinlowm-dkim', {
    zone,
    recordName: 'google._domainkey',
    ttl: Duration.minutes(5),
    values: [GOOGLE_DKIM_ALIAS],
  });

  return zone;
}
