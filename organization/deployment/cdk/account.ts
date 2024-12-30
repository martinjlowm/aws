import assert from 'node:assert';
import { Account, type AccountProps } from '@pepperize/cdk-organizations';
import type { Construct } from 'constructs';

export default class extends Account {
  constructor(scope: Construct, id: string, props: Omit<AccountProps, 'email'>) {
    assert(props.parent, 'Must provide account parent');

    const pathParts = props.parent.node.path.split('/').slice(1).map((p) => p.substring(0, 3).toLowerCase());

    super(scope, id, {
      ...props,
      email: `martinjlowm+${['aws', ...pathParts, id.substring(0, 3).toLowerCase()].join('-')}@gmail.com`,
    });
  }
}
