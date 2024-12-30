import { FeatureSet, Organization } from '@pepperize/cdk-organizations';
import type { Construct } from 'constructs';

import Applications from './applications/index';
import Operations from './operations/index';

export default class extends Organization {
  constructor(scope: Construct) {
    super(scope, 'martinjlowm', {
      featureSet: FeatureSet.ALL,
    });

    this.enableAwsServiceAccess('member.org.stacksets.cloudformation.amazonaws.com');

    new Applications(scope, { parent: this.root });
    new Operations(scope, { parent: this.root });
  }
}
