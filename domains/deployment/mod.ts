import { App } from '@martinjlowm/aws-constructs';

import { Zones } from './stacks/zones';

// Route 53 is global, but the resources declaring it have to sit in some region,
// and that region is the one everything else here sits in.
export default function (app = new App('domains'), region = 'eu-west-1') {
  new Zones(app, app.name, { region });
}
