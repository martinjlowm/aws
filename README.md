# Me on AWS

My personal Infrastructure as Code, IaC, for applications, operational tools and
other goofy projects on AWS.

The `organization/` project can be deployed from a clean root account to set up
the necessary accounts. It expects AWS Organizations to be enabled with
delegated StackSet privileges.

Secondly, the CDK bootstrap (`bootstrap/`) project be deployed such that any new accounts added
to the organization will be bootstrapped automatically.
