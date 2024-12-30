import { OrganizationalUnit, type OrganizationalUnitProps } from '@pepperize/cdk-organizations';
import type { Construct } from 'constructs';

import Account from '../../account';

export default class extends OrganizationalUnit {
  constructor(scope: Construct, props: Pick<OrganizationalUnitProps, 'parent'>) {
    super(scope, 'Music', {
      organizationalUnitName: 'Music',
      parent: props.parent,
    });

    new Account(scope, 'Storage', {
      accountName: 'music-storage',
      parent: this,
    });
  }
}
