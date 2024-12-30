# Me on AWS

My personal Infrastructure as Code, IaC, for applications, operational tools and
other goofy projects on AWS.

The `organization/` project can be deployed from a clean root account to set up
the necessary accounts. It expects AWS Organizations to be enabled with
delegated StackSet privileges.

Secondly, the CDK bootstrap (`bootstrap/`) project be deployed such that any new accounts added
to the organization will be bootstrapped automatically.


## Getting Started

1. Install Nix, direnv and devenv
2. Enter the project and mark the .envrc safe with `direnv allow`
3. Assume a privileged role for your management account with `assume <profile>`
4. Populate `AWS_ORG` (environmental variable) with the organizational structure
   by reloading direnv: `direnv reload`.
