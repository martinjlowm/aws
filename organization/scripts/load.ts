import assert from 'assert';
import camelCase from 'lodash.camelcase';
import { Organizations, ListOrganizationalUnitsForParentCommand, ListRootsCommand, DescribeOrganizationCommand, ListAccountsCommand } from '@aws-sdk/client-organizations';
import { organizationIdentifiersSchema, type OrganizationIdentifiers } from '../src/types';

delete process.env.AWS_PROFILE;

const client = new Organizations();

const { Organization } = await client.send(new DescribeOrganizationCommand());
assert(Organization, 'Failed to describe organization');

const { Roots } = await client.send(new ListRootsCommand());
assert(Roots, 'Failed to list organization roots');

async function populateHierarchyOrganizationalUnits(hierarchy: Record<string, string>, parentId: string) {
  const { OrganizationalUnits } = await client.send(new ListOrganizationalUnitsForParentCommand({ ParentId: parentId }));
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
  Accounts.reduce((obj, account) => {
    obj.accounts[camelCase(account.Name!)] = account.Id;

    return obj;
  }, Roots.reduce((obj, root) => {
    obj[camelCase(root.Name!)] = root.Id;
    return obj;
  }, { roots: {}, organization: Organization.Id, organizationalUnits, accounts: {} })),
);

if (!result.success) {
  console.warn(`The reported organization structure is out of sync with the
  specified schema. Update it to account for absent accounts and/or
organizational units`);
  console.error(result.error);
  process.exit(1);
}

console.info(JSON.stringify(result.data));
