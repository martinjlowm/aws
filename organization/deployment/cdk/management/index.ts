import { OrganizationalUnit, type OrganizationalUnitProps } from '@pepperize/cdk-organizations';
import {
  AwsCustomResource,
  AwsCustomResourcePolicy,
  PhysicalResourceId,
} from 'aws-cdk-lib/custom-resources';
import type { Construct } from 'constructs';

import Account from '../account';

interface ManagementProps extends Pick<OrganizationalUnitProps, 'parent'> {
  /** The organization's own account, which this unit adopts. */
  managementAccountId: string;
  /** Where that account sits until it is moved: the root. */
  rootId: string;
}

/**
 * The accounts that are about the organization rather than about a workload.
 *
 * Structure, not policy. A service control policy attached here would not apply
 * to the management account, whatever unit it sits in, so this unit groups and
 * does not govern. The units that do govern are Operations and Applications.
 */
export default class extends OrganizationalUnit {
  constructor(scope: Construct, props: ManagementProps) {
    super(scope, 'Management', {
      organizationalUnitName: 'Management',
      parent: props.parent,
    });

    // Every domain, and every zone under them. Here rather than in Applications
    // because a domain outlives any one application: mail is delegated from
    // these zones, and an application account that owns an apex is an
    // application whose deletion takes the mail with it.
    //
    // One account for all of them rather than one per domain. Splitting them
    // would separate two zones that carry the same mail to the same mailbox, and
    // what is worth isolating from an application is already isolated by this
    // account existing at all.
    new Account(scope, 'Domains', {
      accountName: 'domains',
      parent: this,
    });

    // The management account, which already exists and is only being moved.
    //
    // Not an `Account`: that construct adopts an existing account by matching
    // name *and* email, and the management account's root address is not the one
    // `../account.ts` derives for every account it creates. Declaring it that way
    // would mean writing that address into this repository to satisfy a match.
    // `MoveAccount` needs neither.
    //
    // Organizations answers on one global endpoint, in us-east-1, wherever this
    // stack is deployed.
    new AwsCustomResource(scope, 'MoveManagementAccount', {
      onUpdate: {
        service: 'Organizations',
        action: 'moveAccount',
        region: 'us-east-1',
        parameters: {
          AccountId: props.managementAccountId,
          SourceParentId: props.rootId,
          DestinationParentId: this.organizationalUnitId,
        },
        // Fixed, so a redeploy that changes nothing does not move an account
        // that is already where it belongs.
        physicalResourceId: PhysicalResourceId.of(`move-${props.managementAccountId}`),
        // Both of these mean the account is already in this unit, which is the
        // state this resource exists to reach. A stack recreated against an
        // organization that has already been moved must not fail on the way in.
        ignoreErrorCodesMatching: 'DuplicateAccountException|SourceParentNotFoundException',
      },
      policy: AwsCustomResourcePolicy.fromSdkCalls({
        resources: AwsCustomResourcePolicy.ANY_RESOURCE,
      }),
    });
  }
}
