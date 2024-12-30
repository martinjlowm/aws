import { App } from '@martinjlowm/aws-constructs';

import { OrganizationStack } from './stacks/index';

export default function (app = new App('org'), region = 'eu-west-1') {
  new OrganizationStack(app, region);
}
