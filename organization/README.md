# AWS Organizations Structure

The initial deployment must be done directly on the management account - any
further updates can be done through a delegated administrator account (not yet
set up here).

Bootstrap happens using the btsrpv1 qualifier such that the bootstrap stack
doesn't conflict with StackSets across all accounts.
