import { App } from '@martinjlowm/aws-constructs';

import { Dns } from './stacks/dns';
import { Site } from './stacks/site';

// eu-west-1 for the bucket, which is where the rest of this organization lives.
// The distribution in front of it is global whatever region declares it, and its
// certificate has to be in us-east-1, which is why the DNS half is its own stack.
export default function (app = new App('dubplate'), region = 'eu-west-1') {
  const dns = new Dns(app, app.name);

  new Site(app, app.name, {
    region,
    zone: dns.zone,
    certificate: dns.certificate,
  });
}
