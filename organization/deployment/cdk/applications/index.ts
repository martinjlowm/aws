import { OrganizationalUnit, type OrganizationalUnitProps } from '@pepperize/cdk-organizations';
import type { Construct } from 'constructs';

import Music from './music/index';

export default class extends OrganizationalUnit {
  constructor(scope: Construct, props: Pick<OrganizationalUnitProps, 'parent'>) {
    super(scope, 'Applications', {
      organizationalUnitName: 'Applications',
      parent: props.parent,
    });

    new Music(scope, { parent: this });
  }
}
