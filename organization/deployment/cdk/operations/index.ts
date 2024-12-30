import { OrganizationalUnit, type OrganizationalUnitProps } from '@pepperize/cdk-organizations';
import { Role, ManagedPolicy, AccountPrincipal } from 'aws-cdk-lib/aws-iam';
import type { Construct } from 'constructs';

import Account from '../account';

export default class extends OrganizationalUnit {
  constructor(scope: Construct, props: Pick<OrganizationalUnitProps, 'parent'>) {
    super(scope, 'Operations', {
      organizationalUnitName: 'Operations',
      parent: props.parent,
    });

    const cdkBootstrap = new Account(scope, 'CDKBootstrap', {
      accountName: 'cdk-bootstrap',
      parent: this,
     });

    new Role(scope, 'StackSetExecutionRole', {
      roleName: 'AWSCloudFormationStackSetExecutionRole',
      managedPolicies: [ManagedPolicy.fromAwsManagedPolicyName('AdministratorAccess')],
      assumedBy: new AccountPrincipal(cdkBootstrap.accountId),
    });

    cdkBootstrap.delegateAdministrator('member.org.stacksets.cloudformation.amazonaws.com');
  }
}
