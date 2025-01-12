import { Stack, Duration, type StackProps } from 'aws-cdk-lib';
import { Bucket, type IBucket, StorageClass } from 'aws-cdk-lib/aws-s3';
import { Construct } from 'constructs';

export class S3 extends Stack {
  bucket: IBucket;

  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, `${id}-s3`, props);

    this.bucket = new Bucket(this, 'bucket', {
      bucketName: `${this.account}-${this.region}-music`,
      lifecycleRules: [
        {
          id: 'lifecycle-ia-rule',
          enabled: true,
          transitions: [
            {
              storageClass: StorageClass.INFREQUENT_ACCESS,
              transitionAfter: Duration.days(30),
            }
          ]
        },
      ],
    });
  }
}
