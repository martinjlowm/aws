import assert from 'assert';
import { AccountPrincipal, ManagedPolicy, Role } from 'aws-cdk-lib/aws-iam';
import { dirname } from 'node:path';
import { Stack } from 'aws-cdk-lib';
import { CfnInclude } from 'aws-cdk-lib/cloudformation-include';
import { App } from '@martinjlowm/aws-constructs';
import {
  Capability,
  DeploymentType,
  RegionConcurrencyType,
  StackSet,
  StackSetStack,
  StackSetTarget,
  StackSetTemplate,
} from 'cdk-stacksets';

import { parse, organizationIdentifiersSchema } from '@martinjlowm/aws-organization';

const org = parse(process.env.AWS_ORG, organizationIdentifiersSchema);

export class CDKBootstrapStack extends Stack {
  constructor(app: App, region: string) {
    super(app, `${app.name}-cdk`, { env: { region, account: org.accounts.cdkBootstrap, }});

    const bootstrap = new StackSetStack(this, 'StackSetStack');

    new CfnInclude(bootstrap, 'Template', {
      templateFile: `${dirname(require.resolve('aws-cdk/package.json'))}/lib/api/bootstrap/bootstrap-template.yaml`,
    });

    new Role(this, 'StackSetExecutionRole', {
      roleName: 'AWSCloudFormationStackSetExecutionRole',
      managedPolicies: [ManagedPolicy.fromAwsManagedPolicyName('AdministratorAccess')],
      assumedBy: new AccountPrincipal(org.accounts.cdkBootstrap),
    });

    new StackSet(this, 'cdk-bootstrap-service-managed', {
      stackSetName: 'CDKToolkit-service-managed',
      target: StackSetTarget.fromOrganizationalUnits({
        regions: ['us-east-1', 'eu-west-1'],
        organizationalUnits: [org.organizationalUnits.applications, org.organizationalUnits.operations],
      }),
      deploymentType: DeploymentType.serviceManaged({
        delegatedAdmin: true,
        autoDeployEnabled: true,
        autoDeployRetainStacks: false,
      }),
      operationPreferences: {
        regionConcurrencyType: RegionConcurrencyType.PARALLEL,
        maxConcurrentPercentage: 100,
        failureTolerancePercentage: 99,
      },
      template: StackSetTemplate.fromStackSetStack(bootstrap),
      capabilities: [Capability.NAMED_IAM],
    });

    // This specifically covers our MGMT account from which we have to be
    // explicit about deployment
    const selfManaged = new StackSet(this, 'cdk-bootstrap-self-managed', {
      stackSetName: 'CDKToolkit-self-managed',
      target: StackSetTarget.fromAccounts({
        regions: ['us-east-1', 'eu-west-1'],
        accounts: [org.accounts.management],
      }),
      operationPreferences: {
        regionConcurrencyType: RegionConcurrencyType.PARALLEL,
        maxConcurrentPercentage: 100,
        failureTolerancePercentage: 99,
      },
      template: StackSetTemplate.fromStackSetStack(bootstrap),
      capabilities: [Capability.NAMED_IAM],
    });

    if (selfManaged.role) {
      selfManaged.node.addDependency(selfManaged.role);
    }
  }
}
