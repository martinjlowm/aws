import { Certificate, CertificateValidation } from 'aws-cdk-lib/aws-certificatemanager';
import { Role } from 'aws-cdk-lib/aws-iam';
import { CrossAccountZoneDelegationRecord, PublicHostedZone } from 'aws-cdk-lib/aws-route53';
import { Construct } from 'constructs';

/**
 * The role every domain's zone grants delegation to, in the domains account.
 *
 * Declared here rather than in `aws/domains` because both ends need the name and
 * neither should depend on the other: the account that writes the delegation has
 * to know what to assume before the account that holds the zone has told it
 * anything.
 */
export const ZONE_DELEGATION_ROLE = 'ZoneDelegation';

export interface DelegatedZoneProps {
  /** The subdomain this application answers for, e.g. `dubplate.martinjlowm.dk`. */
  readonly domainName: string;
  /** The account holding the parent zone. */
  readonly domainsAccountId: string;
  /**
   * The zone the delegation is written into. Defaults to `domainName` with its
   * first label removed, which is right for a subdomain of a domain we own and
   * wrong for anything deeper.
   */
  readonly parentDomainName?: string;
  /**
   * Also issue a certificate for the zone, validated against it.
   *
   * @default false
   */
  readonly certificate?: boolean;
}

/**
 * A subdomain an application owns outright.
 *
 * The zone is created in the application's own account and an NS record pointing
 * at it is written into the parent zone in the domains account. From then on the
 * application adds hostnames by deploying itself: nothing it does reaches the
 * zone that carries the mail, and the worst a mistake can do is break the
 * application making it.
 *
 * The role this assumes trusts the whole organization and permits `UPSERT` and
 * `DELETE` of `NS` records and nothing else, so no deployment in `aws/domains`
 * is needed before a new application can use this.
 *
 * With `certificate`, the stack this is declared in decides where the
 * certificate lives. CloudFront reads certificates from us-east-1 and nowhere
 * else, so an application in front of a distribution declares this in a
 * us-east-1 stack and reads the result from wherever the distribution is.
 */
export class DelegatedZone extends Construct {
  readonly zone: PublicHostedZone;
  readonly certificate?: Certificate;

  constructor(scope: Construct, id: string, props: DelegatedZoneProps) {
    super(scope, id);

    const parent = props.parentDomainName ?? props.domainName.split('.').slice(1).join('.');
    if (!parent.includes('.')) {
      throw new Error(
        `${props.domainName} has no parent zone to delegate from; it is a domain, not a subdomain`,
      );
    }

    this.zone = new PublicHostedZone(this, 'zone', {
      zoneName: props.domainName,
      comment: `Delegated from ${parent}.`,
    });

    // The NS record that makes the delegation real. Without it this zone answers
    // for a name nobody is told to ask it about.
    new CrossAccountZoneDelegationRecord(this, 'delegation', {
      delegatedZone: this.zone,
      parentHostedZoneName: parent,
      delegationRole: Role.fromRoleArn(
        this,
        'delegation-role',
        `arn:aws:iam::${props.domainsAccountId}:role/${ZONE_DELEGATION_ROLE}`,
      ),
    });

    if (props.certificate) {
      // Validated against the zone above, which this construct has just created,
      // so the record lands without anybody copying a string between consoles.
      this.certificate = new Certificate(this, 'certificate', {
        domainName: props.domainName,
        validation: CertificateValidation.fromDns(this.zone),
      });
    }
  }
}
