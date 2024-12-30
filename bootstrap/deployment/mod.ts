import { App } from '@martinjlowm/aws-constructs';

import { CDKBootstrapStack } from './stacks/index';

export default function (app = new App('bootstrap'), region = 'eu-west-1') {
  new CDKBootstrapStack(app, region);
}
