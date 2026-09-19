import type { App } from '@martinjlowm/aws-constructs';
import { organizationIdentifiersSchema, parse } from '@martinjlowm/aws-organization';
import { Stack } from 'aws-cdk-lib';
import { CfnBudget } from 'aws-cdk-lib/aws-budgets';
import { CfnAnomalyMonitor, CfnAnomalySubscription } from 'aws-cdk-lib/aws-ce';

const org = parse(process.env.AWS_ORG, organizationIdentifiersSchema);

/**
 * Where both alerts are sent.
 *
 * A plus alias on the same mailbox `organization/deployment/cdk/account.ts`
 * gives every account it creates, so the alerts land beside the root mail AWS
 * already sends and filter on their own address. Not the management account's
 * root address, which this repository does not hold and does not need to: a
 * subscriber is a destination, not proof of who owns the account.
 */
const ALERT_EMAIL = 'martinjlowm+aws-billing@gmail.com';

/** What a month is expected to cost, in USD. */
const MONTHLY_LIMIT = 20;

/**
 * The smallest anomaly worth an email, in USD.
 *
 * Absolute rather than a percentage. A bill this size spends single digits on
 * most services, and a percentage threshold on a service costing cents reports
 * a tenfold increase that is still cents.
 */
const ANOMALY_IMPACT = 5;

/**
 * The two ways the bill says something changed, both of which email
 * `ALERT_EMAIL`.
 *
 * They answer different questions and neither replaces the other. Anomaly
 * detection compares a service against what that service usually costs, so it
 * catches one service moving however small the total stays. The budget compares
 * the total against a number written here, so it catches a total that is growing
 * without any single service looking unusual.
 *
 * Both are read-only over billing data and neither can stop a charge. What they
 * buy is the days between a change and the invoice.
 */
export class Alerts extends Stack {
  constructor(app: App, id: string) {
    super(app, `${id}-alerts`, {
      env: { region: 'us-east-1', account: org.accounts.management },
    });

    // A monitor in the management account reads the consolidated bill, so every
    // member account's services are in scope without this stack naming any of
    // them. Adding an account to the organization is not a change here.
    const services = new CfnAnomalyMonitor(this, 'services', {
      monitorName: 'services',
      monitorType: 'DIMENSIONAL',
      monitorDimension: 'SERVICE',
    });

    // Daily rather than immediate. An immediate subscription has to deliver to
    // an SNS topic, and a topic whose only subscriber is this mailbox is a
    // resource to maintain for a delay measured in hours on a bill measured in
    // tens of dollars.
    new CfnAnomalySubscription(this, 'anomalies', {
      subscriptionName: 'anomalies',
      frequency: 'DAILY',
      monitorArnList: [services.attrMonitorArn],
      subscribers: [{ type: 'EMAIL', address: ALERT_EMAIL }],
      // A JSON string, not a structure. CloudFormation types this property as
      // one, and the shape inside it is a Cost Explorer expression.
      thresholdExpression: JSON.stringify({
        Dimensions: {
          Key: 'ANOMALY_TOTAL_IMPACT_ABSOLUTE',
          MatchOptions: ['GREATER_THAN_OR_EQUAL'],
          Values: [`${ANOMALY_IMPACT}`],
        },
      }),
    });

    new CfnBudget(this, 'monthly', {
      budget: {
        budgetName: 'monthly',
        budgetType: 'COST',
        timeUnit: 'MONTHLY',
        budgetLimit: { amount: MONTHLY_LIMIT, unit: 'USD' },
      },
      notificationsWithSubscribers: [
        // The forecast first, because it is the only one that can arrive before
        // the money is spent. Budgets projects the month from what has run so
        // far, so a resource left on overnight shows up here on the morning
        // after rather than in the invoice three weeks later.
        notify('FORECASTED', 100),
        // Then the month as it actually stands. Eighty per cent is the warning
        // and a hundred is the fact; fifty is neither, and on a bill this size
        // it would fire most months on nothing but the steady cost of running.
        notify('ACTUAL', 80),
        notify('ACTUAL', 100),
      ],
    });
  }
}

/** One threshold on the budget, emailed to `ALERT_EMAIL`. */
function notify(
  notificationType: 'ACTUAL' | 'FORECASTED',
  threshold: number,
): CfnBudget.NotificationWithSubscribersProperty {
  return {
    notification: {
      notificationType,
      comparisonOperator: 'GREATER_THAN',
      thresholdType: 'PERCENTAGE',
      threshold,
    },
    subscribers: [{ subscriptionType: 'EMAIL', address: ALERT_EMAIL }],
  };
}
