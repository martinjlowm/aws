import { FeatureSet, Organization } from '@pepperize/cdk-organizations';
import type { Construct } from 'constructs';

import Applications from './applications/index';

export default class extends Organization {
  constructor(scope: Construct) {
    super(scope, 'martinjlowm', {
      featureSet: FeatureSet.ALL,
    });

    new Applications(this, { parent: this.root });
  }
}
