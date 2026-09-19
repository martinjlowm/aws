import { OrganizationalUnit, type OrganizationalUnitProps } from '@pepperize/cdk-organizations';
import type { Construct } from 'constructs';

import Account from '../../account';

export default class extends OrganizationalUnit {
  constructor(scope: Construct, props: Pick<OrganizationalUnitProps, 'parent'>) {
    super(scope, 'Music', {
      organizationalUnitName: 'Music',
      parent: props.parent,
    });

    // The bundles as they were downloaded.
    new Account(scope, 'Storage', {
      accountName: 'music-storage',
      parent: this,
    });

    // The bucket and the distribution that serve dubplate. Here rather than in a
    // unit of its own because it is the same subject as the account above: one
    // stores the archives, the other measures what is in them. A unit per
    // application would be a unit per account, which groups nothing.
    new Account(scope, 'Dubplate', {
      accountName: 'dubplate-web',
      parent: this,
    });
  }
}
