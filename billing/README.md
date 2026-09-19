# Billing

Two alerts on the consolidated bill, in the management account: Cost Explorer
anomaly detection, and a monthly budget. Both email `ALERT_EMAIL`.

Neither can stop a charge. What they buy is the days between a cost changing and
the invoice that would otherwise be the first news of it.

---

## Tutorial

Turning the alerts on, from an organization that has never opened Cost Explorer.

1. **Enable Cost Explorer.** There is no API and no CloudFormation resource for
   this. It turns on the first time you open the console page, and
   `AWS::CE::AnomalyMonitor` cannot be created before it is on.

   ```sh
   assume <management profile>
   open https://console.aws.amazon.com/costmanagement/home#/anomaly-detection
   ```

   AWS then backfills up to twelve months of history, which takes up to 24
   hours. Deploying before that finishes fails on the monitor.

2. **Deploy.**

   ```sh
   just deploy billing
   ```

3. **Confirm nothing needs confirming.** Both services send email directly, so
   there is no subscription to accept and no first message to wait for. The
   budget is visible immediately:

   ```sh
   aws budgets describe-budgets --account-id "$(aws sts get-caller-identity \
     --query Account --output text)" --query 'Budgets[].BudgetName'
   ```

4. **Wait about ten days for the anomaly monitor.** It reports a service against
   what that service usually costs, and it has no idea what that is until it has
   watched for a while. Silence before then is the monitor learning, not the
   bill being quiet.

---

## How-to guides

### Change what a month is expected to cost

`MONTHLY_LIMIT` in `deployment/stacks/alerts.ts`, then `just deploy billing`.
The thresholds are percentages of it, so they follow.

### Change how small an anomaly can be and still send mail

`ANOMALY_IMPACT`, in the same file. It is dollars of impact, not a percentage:
see the explanation below for why.

### Send the alerts somewhere else

`ALERT_EMAIL` in `deployment/stacks/alerts.ts`. Both subscriber lists read it,
so one edit moves both. A second recipient is a second entry in each list.

For anything richer than email, both resources take an SNS topic instead of an
address: `Type: 'SNS'` on the anomaly subscriber, `subscriptionType: 'SNS'` on a
budget subscriber. An SNS subscription also unlocks `IMMEDIATE` frequency on the
anomaly subscription, which email cannot have.

### Alert on one account rather than the whole bill

Add a second `CfnBudget` with `costFilters: { LinkedAccount: [<id>] }`. Anomaly
detection does not split the same way: `MonitorDimension` accepts `SERVICE` and
nothing else, and per-account monitoring is a `CUSTOM` monitor over a cost
category that does not exist here yet.

---

## Reference

### What this deploys

| Resource | Type | What it watches |
|---|---|---|
| `services` | `AWS::CE::AnomalyMonitor` | every service on the consolidated bill, against its own history |
| `anomalies` | `AWS::CE::AnomalySubscription` | that monitor, mailing once a day when impact reaches $5 |
| `monthly` | `AWS::Budgets::Budget` | the month's total against $20 |

### The budget's thresholds

| Type | Threshold | Arrives |
|---|---|---|
| Forecasted | 100% | when the projected month first crosses the limit |
| Actual | 80% | once four fifths of the limit is spent |
| Actual | 100% | once the limit is spent |

The forecast is the only one that can arrive before the money is gone. Budgets
projects the month from what has run so far, so a resource left on overnight
shows up the morning after rather than three weeks later.

There is no 50% threshold. On a bill this size it would fire most months on
nothing but the steady cost of running, and an alert that is usually noise is an
alert nobody reads.

### Region

`us-east-1`, hard-coded rather than a parameter. Cost Explorer and Budgets answer
on one global endpoint there, and `AWS::CE::AnomalyMonitor` exists in no other
region. Every other project here takes a region because it has a choice.

### Where the address comes from

`ALERT_EMAIL` in `deployment/stacks/alerts.ts`, written down rather than looked
up. It is a plus alias on the mailbox `organization/deployment/cdk/account.ts`
already gives every account it creates, so the alerts arrive beside the root
mail AWS sends and filter on their own address.

Not the management account's root address. A subscriber is a destination, not
proof of who owns the account, so nothing here needs the one address
`organization/README.md` says the derivation cannot produce.

---

## Explanation

### Why both, when either sounds like enough

They answer different questions and neither subsumes the other.

Anomaly detection compares a service against what that service usually costs. It
catches one service moving while the total stays unremarkable, which is the shape
of nearly every accident: a NAT gateway, a forgotten instance, a bucket being
crawled. It has no opinion about the total.

The budget compares the total against a number written down here. It catches a
total that is climbing without any single service looking unusual, which is what
growth looks like, and it catches the first month of something anomaly detection
has no history for.

Drop either one and its half of that goes unreported.

### Why the anomaly threshold is dollars, not a percentage

Cost Explorer offers both, and on a bill this size the percentage is useless. A
service costing forty cents a month tripling is a 200% anomaly and twelve
hundredths of a dollar; a threshold that catches it sends mail every week about
nothing. Five dollars of impact is small against a twenty dollar month and large
against the noise.

The trade is that a percentage threshold scales with the bill and this does not.
That is a line to change in `alerts.ts` when the bill has changed enough to
notice, which is later than it sounds.

### Why daily rather than immediate

`IMMEDIATE` frequency requires an SNS subscriber; an email subscriber can have
`DAILY` or `WEEKLY`. Getting immediate mail therefore means a topic, an email
subscription on it, and a confirmation click, all so that a bill measured in
tens of dollars is reported in minutes instead of hours.

Nothing here is actionable in that window. Daily is the summary that arrives
before the day's spend is a day old, and it costs one resource.

### Why the management account and no other

A monitor in the management account reads the consolidated bill, so every member
account's services are in scope without this stack naming any of them. Adding an
account to the organization is not a change here, which is the same reason the
bootstrap stack set targets units rather than accounts.

It also has to be here. Member accounts see only their own costs, and a budget
in each one is a budget per account that nobody reconciles into the number that
actually gets charged.
