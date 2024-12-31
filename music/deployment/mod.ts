import { Aspects, Stack } from 'aws-cdk-lib';
import { AwsSolutionsChecks } from 'cdk-nag';
import { App } from '@martinjlowm/aws-constructs';

import { S3 } from './stacks/s3';

export default function (app = new App('music-storage'), region = 'eu-west-1') {
  new S3(app, app.name, { env: { region }});
}
