# Domains

Every domain, in one `domains` account under the Management unit:
`martinjlowm.dk` and `martinlowm.dk`.

Only records belonging to a domain itself live here, which today is mail and
verification. Everything an application serves is a delegated subdomain in that
application's own account, so deploying one cannot touch the MX records and
deleting one cannot take them with it.

---

## Tutorial

Moving both domains off Cloudflare, in the order that keeps mail working.

1. **Create the account and deploy the zones.**

   ```sh
   just deploy organization   # creates the domains account
   direnv reload              # AWS_ORG learns about it
   assume aws-man-dom
   just deploy domains
   ```

2. **Read the nameservers AWS assigned.** Every zone gets four, they differ per
   zone, and they cannot be written down here.

   ```sh
   for domain in martinjlowm.dk martinlowm.dk; do
     echo "== $domain"
     aws route53 get-hosted-zone --id "$(aws route53 list-hosted-zones-by-name \
       --dns-name "$domain" --query 'HostedZones[0].Id' --output text)" \
       --query 'DelegationSet.NameServers' --output text
   done
   ```

3. **Check each zone answers before anyone is pointed at it.** Ask one of its
   own nameservers directly, which works while the delegation still says
   Cloudflare:

   ```sh
   dig MX martinjlowm.dk @ns-1234.awsdns-56.org +noall +answer
   ```

   Five Google records, or stop here and find out why.

4. **Shorten the old TTLs.** In Cloudflare, drop every record to 60 seconds and
   wait for the old values to expire. This is what makes step 5 reversible in a
   minute rather than a day.

5. **Point the registrars at AWS.** `martinjlowm.dk` is registered through
   Punktum.dk and `martinlowm.dk` through Simply.com; each takes its own four
   nameservers.

   Neither domain is signed (`DNSSEC: Unsigned delegation` in both registry
   records), so there is no DS to remove first and no window where validation
   fails. Had there been one, removing it and waiting out its TTL would come
   before this step.

6. **Watch mail.** The delegation takes minutes to hours.

   ```sh
   dig MX martinjlowm.dk @1.1.1.1 +noall +answer
   ```

   Google retries delivery for days, so slow propagation loses nothing.

---

## How-to guides

### Add a record for a domain itself

Edit the relevant function in `deployment/stacks/zones.ts` and deploy. Records
added by hand in the console are removed the next time this stack deploys, which
is the point of declaring them.

### Give an application its own subdomain

Do not add a record here, and do not deploy this stack. The role trusts the whole
organization, so an application delegates itself:

```ts
import { DelegatedZone } from '@martinjlowm/aws-constructs';

const delegated = new DelegatedZone(this, 'thing', {
  domainName: 'thing.martinjlowm.dk',
  domainsAccountId: org.accounts.domains,
  // Optional. CloudFront reads certificates from us-east-1 and nowhere else, so
  // a stack that wants one for a distribution declares it there.
  certificate: true,
});
```

That creates the zone in the application's own account, writes the NS record into
the parent here, and hands back `delegated.zone` and `delegated.certificate`.
`dubplate/deployment/stacks/dns.ts` is the worked example.

The parent defaults to the domain with its first label removed, which is right
for a subdomain of a domain we own. Pass `parentDomainName` for anything deeper.

### Add a domain

A new zone and a new call to `grantDelegation` in `zones.ts`. Both domains here
deliver to the same Google Workspace mailbox and so carry the same MX records,
which is why they are a constant rather than two copies.

### Publish SPF, DKIM and DMARC

Neither domain has a TXT record asserting who may send as it: mail leaves
Google with nothing behind it. Adding them is three records per domain in
`zones.ts`. The DKIM value comes from the Google Workspace admin console and
cannot be written down ahead of it.

---

## Reference

### What the zones hold

**martinjlowm.dk**

| Name | Type | TTL | Value |
|---|---|---|---|
| `martinjlowm.dk` | MX | 300 | Google Workspace, priorities 1, 5, 5, 10, 10 |

**martinlowm.dk**

| Name | Type | TTL | Value |
|---|---|---|---|
| `martinlowm.dk` | MX | 300 | the same five |
| `martinlowm.dk` | TXT | 300 | `google-site-verification=nTDBdrWiI8w1eD0XyhNxJ7PoIl1c3VRJbq4uIgPEHWQ` |

Every TTL and value is what Cloudflare served. Mail is the one thing already
working, and a migration is not the moment to change it.

### What the zones do not hold

The apex and `www` on both domains were Cloudflare proxy addresses in front of a
Heroku application that no longer runs. Reproducing them would be a zone that
answers and a site that does not, so they are absent rather than guessed at.

### The delegation role

`ZoneDelegation`, in this account.

| | |
|---|---|
| Who may assume it | any principal in this organization, by `aws:PrincipalOrgID` |
| What it may do | `UPSERT` and `DELETE` of `NS` records, in these two zones |
| What it may not do | touch any other record type, in any zone, ever |

The name is `ZONE_DELEGATION_ROLE` in `@martinjlowm/aws-constructs`, which both
ends import. Neither this stack nor an application's stack writes the string, so
the two cannot drift.

---

## Explanation

### Why one account for every domain

The domains outlive any one application. Mail is delegated from both apexes, and
an application account that owns an apex is an application whose deletion takes
the mail with it. The Management unit is where the accounts that are about the
organization rather than about a workload sit, which is what a domain is.

One account for both domains rather than one each: what splitting them would
separate is two zones carrying the same mail to the same mailbox, and what is
worth isolating from an application is already isolated by this account existing.

### Why this is not the management account

It sits beside the management account and is not it. The management account pays
the bill and owns the organization; a zone in it would be a zone nobody can
delegate access to without handing out access to the organization itself.

The unit they share attaches no policy and could not: service control policies
never apply to the management account, whatever unit it sits in. It groups.
`organization/README.md` says so where somebody would otherwise attach one.

### Why the role trusts the whole organization

Broad trust with narrow permission, rather than the reverse.

Naming each account in the trust means deploying this stack before every new
application, which makes the shared zone a step in unrelated work and eventually
makes someone add a record by hand instead. Trusting the organization removes
that step.

It is safe because of what the role can do, not who holds it: `grantDelegation`
permits `UPSERT` and `DELETE` of `NS` records and nothing else. An account that
assumes it can delegate a subdomain. It cannot change an MX record, so the worst
a compromised application account can do here is misdirect its own hostname, and
the mail is out of reach.

What this does not prevent is one application delegating a name another was going
to use. That is a collision between two things the organization already trusts,
and if it ever matters the fix is `delegatedZoneNames` on the grant, which pins
each account to the names it owns.

### Why subdomains are delegated rather than written here

A record in these zones is a change to the zones that carry the mail. A delegated
subdomain is a zone of its own in the account that serves it: the application
adds hostnames by deploying itself, and the worst a mistake there can do is break
that application. It also means ACM validates a certificate against a zone in the
same account, so a deployment does not stop to have a string copied out of one
console into another.

The cost is one extra level of delegation, which a resolver walks inside a query
it was already making.
