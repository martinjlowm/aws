import { App } from '@martinjlowm/aws-constructs';

import { Alerts } from './stacks/alerts';

// Cost Explorer and Budgets answer on one global endpoint, in us-east-1, and
// `AWS::CE::AnomalyMonitor` exists in no other region. Not a parameter for that
// reason, unlike every other project here.
export default function (app = new App('billing')) {
  new Alerts(app, app.name);
}
