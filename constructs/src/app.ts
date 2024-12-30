import { Aspects, App as CDKApp, type AppProps, Stack } from 'aws-cdk-lib';
import { AwsSolutionsChecks } from 'cdk-nag';

export class App extends CDKApp {
  name: string;

  constructor(name: string) {
    super({ context: {
      "@aws-cdk/aws-lambda:recognizeVersionProps": true,
      "@aws-cdk/customresources:installLatestAwsSdkDefault": false
    }});

    this.name = name;

    // Aspects.of(this).add(new AwsSolutionsChecks({ verbose: true }));
  }
}
