import { Stack } from 'aws-cdk-lib';
import { NagSuppressions } from 'cdk-nag';
import { App } from '@martinjlowm/aws-constructs';

import Organization from '../cdk/index';

export class OrganizationStack extends Stack {
  constructor(app: App, region: string) {
    super(app, `${app.name}-organizations`, { env: { region }});

    const org = new Organization(this);

    // NagSuppressions.addStackSuppressions(this, [
    //   { id: 'CdkNagValidationFailure', reason: 'lorem ipsum' },
    // ], true);
  }
}
