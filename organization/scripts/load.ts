import {
  DescribeOrganizationCommand,
  ListAccountsCommand,
  ListOrganizationalUnitsForParentCommand,
  ListRootsCommand,
  Organizations,
} from '@aws-sdk/client-organizations';
import assert from 'assert';
import camelCase from 'lodash.camelcase';
import { type OrganizationIdentifiers, organizationIdentifiersSchema } from '../src/types';

delete process.env.AWS_PROFILE;

// `assume` exports the session as AWS_ACCESS_KEY_ID and friends, and the SDK's
// credential chain reads those first. With none of them set the chain walks on
// to the remote providers and spins there rather than failing, and direnv, which
// sources this script, blocks the shell for as long as it runs. Check the
// variables the chain wants before building a client, so an unassumed session is
// a message instead of a hang.
if (!process.env.AWS_ACCESS_KEY_ID || !process.env.AWS_SECRET_ACCESS_KEY) {
  console.error(
    'No AWS credentials in the environment. Assume a privileged role for the management account with `assume <profile>`, then run `direnv reload`.',
  );
  process.exit(1);
}

// Organizations answers on one global endpoint, but the client still refuses to
// send without a region, and resolving one from an empty environment is the
// other half of that hang.
const client = new Organizations({
  region: process.env.AWS_REGION ?? process.env.AWS_DEFAULT_REGION ?? 'us-east-1',
});

const { Organization } = await client.send(new DescribeOrganizationCommand());
assert(Organization, 'Failed to describe organization');

const { Roots } = await client.send(new ListRootsCommand());
assert(Roots, 'Failed to list organization roots');

async function populateHierarchyOrganizationalUnits(
  hierarchy: Record<string, string>,
  parentId: string,
) {
  const { OrganizationalUnits } = await client.send(
    new ListOrganizationalUnitsForParentCommand({ ParentId: parentId }),
  );
  assert(OrganizationalUnits, 'Failed to list accounts');

  for (const ou of OrganizationalUnits) {
    hierarchy[camelCase(ou.Name!)] = ou.Id!;
    await populateHierarchyOrganizationalUnits(hierarchy, ou.Id!);
  }
}

const organizationalUnits = {};
for (const root of Roots) {
  await populateHierarchyOrganizationalUnits(organizationalUnits, root.Id!);
}

const { Accounts } = await client.send(new ListAccountsCommand());
assert(Accounts, 'Failed to list accounts');

const result = organizationIdentifiersSchema.safeParse(
  Accounts.reduce(
    (obj, account) => {
      obj.accounts[camelCase(account.Name!)] = account.Id;

      return obj;
    },
    Roots.reduce(
      (obj, root) => {
        obj[camelCase(root.Name!)] = root.Id;
        return obj;
      },
      { roots: {}, organization: Organization.Id, organizationalUnits, accounts: {} },
    ),
  ),
);

if (!result.success) {
  // The fields, one per line, rather than a ZodError dump. Nearly every time
  // this fires it is the same thing and it is not an error: the schema names an
  // account that the organization has not been told to create yet, which is the
  // order those two things happen in.
  //
  // stderr, because .envrc reads stdout as the value.
  console.warn(
    [
      'The organization does not match organizationIdentifiersSchema:',
      ...result.error.issues.map((issue) => `  ${issue.path.join('.')}: ${issue.message}`),
      '',
      'Usually the schema is ahead of the organization, which is how a new',
      'account starts. Deploy it and reload:',
      '',
      '  just deploy organization   # the one project that needs no AWS_ORG',
      '  direnv reload',
      '',
      'If a field names something this organization will never have, remove it',
      'from organization/src/types.ts instead.',
    ].join('\n'),
  );
  process.exit(1);
}

console.info(JSON.stringify(result.data));
