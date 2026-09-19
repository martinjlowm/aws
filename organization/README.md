# Organization

The account tree, from a clean root account.

```
Root
├── Management (OU)
│   ├── management            the organization's own account, at the top of the bill
│   └── domains               every hosted zone: martinjlowm.dk, martinlowm.dk
├── Operations (OU)
│   └── cdk-bootstrap         delegated administrator for the bootstrap stack set
└── Applications (OU)
    └── Music (OU)
        ├── music-storage     the Beatport bundles
        └── dubplate-web      the bucket and the distribution serving dubplate
```

---

## How-to guides

### Add an account

A `new Account` in the unit that should hold it, then an `accountSchema` field in
`organizationIdentifiersSchema`. The schema is what `.envrc` validates `AWS_ORG`
against, so a new account makes `direnv reload` fail until it exists, which is
the intended order: deploy this project, then reload, then deploy the project
that uses the account.

---

## Reference

### Emails

`deployment/cdk/account.ts` derives every account's email from where it sits:
`martinjlowm+aws-<unit>-<account>@gmail.com`, each part cut to three letters. The
domains account is `martinjlowm+aws-man-dom@gmail.com`.

An account that moves between units keeps the email it was created with. The
derivation names the address at creation; it is not a lookup.

`AWS_ORG` carries account ids and no addresses. A stack that has to mail
somebody writes the address down, as `billing/` does. The management account is
the one this derivation cannot produce an address for, and nothing needs it to.

### What each unit is for

| Unit | Holds | Governed by policy |
|---|---|---|
| Management | the organization's own account, and what is about the organization rather than a workload | no |
| Operations | the machinery that deploys everything else | yes |
| Applications | one unit per subject, one or more accounts each | yes |

---

## Explanation

### The management account is moved, not created

Every other account here is created by `deployment/cdk/account.ts`, which derives
its email from where it sits. The management account predates all of that and has
a root address the derivation would not produce.

So it is not an `Account`. Adopting an existing account with that construct means
matching it on name **and** email, which would mean writing the root address into
this repository to satisfy a match. `MoveAccount` needs neither: the organization
already knows which account is its own, and `Organization.managementAccountId` is
what the custom resource in `deployment/cdk/management/index.ts` passes it.

It is idempotent. The call ignores `DuplicateAccountException` and
`SourceParentNotFoundException`, both of which mean the account is already in the
unit, so a stack recreated against an organization that has already been moved
does not fail on the way in.

### Management groups, it does not govern

A service control policy attached to the Management unit would not apply to the
management account. That is true of every unit the management account sits in,
and it is why AWS's own guidance leaves it at the root.

The unit is here for structure: it says which accounts are about the
organization rather than about something it runs, and it puts them next to each
other in the console. Nothing is attached to it, and nothing should be attached
to it expecting to constrain what is inside.

The units that do govern are Operations and Applications, both of which hold only
member accounts.

### One unit per subject, not per account

Music holds two accounts: the bucket of Beatport archives, and the site that
measures what is in them. They are one subject with two blast radiuses, which is
what a unit is for.

A unit wrapping a single account groups nothing and costs a level of nesting
every policy and every stack set has to name. Applications gets a new unit when
there is a second thing that is not music, not when there is a second account.

### Why domains sits beside the management account

The domains outlive every application. Mail is delegated from both apexes, and an
application account that owns an apex is an application whose deletion takes the
mail with it. Grouping it with the management account says what it is: a thing
the whole organization depends on, owned by none of it.

The delegation runs the other way, so this costs applications nothing.
`domains/README.md` covers the role that lets any account in the organization
delegate itself a subdomain without anything here being deployed first.
