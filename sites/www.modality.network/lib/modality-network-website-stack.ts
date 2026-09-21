import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as s3deploy from 'aws-cdk-lib/aws-s3-deployment';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as origins from 'aws-cdk-lib/aws-cloudfront-origins';
import * as route53 from 'aws-cdk-lib/aws-route53';
import * as targets from 'aws-cdk-lib/aws-route53-targets';
import * as acm from 'aws-cdk-lib/aws-certificatemanager';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as path from 'path';

export interface ModalityNetworkWebsiteStackProps extends cdk.StackProps {
  domainName: string;
  wwwDomainName: string;
}

export class ModalityNetworkWebsiteStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props: ModalityNetworkWebsiteStackProps) {
    super(scope, id, props);

    const { domainName, wwwDomainName } = props;

    const hostedZone = route53.HostedZone.fromLookup(this, 'HostedZone', {
      domainName,
    });

    const certificate = new acm.Certificate(this, 'Certificate', {
      domainName,
      subjectAlternativeNames: [wwwDomainName],
      validation: acm.CertificateValidation.fromDns(hostedZone),
    });

    const siteBucket = new s3.Bucket(this, 'SiteBucket', {
      bucketName: `${wwwDomainName}-content`,
      publicReadAccess: false,
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      autoDeleteObjects: false,
      versioned: true,
      encryption: s3.BucketEncryption.S3_MANAGED,
      enforceSSL: true,
    });

    const oai = new cloudfront.OriginAccessIdentity(this, 'OAI', {
      comment: `OAI for ${domainName}`,
    });

    siteBucket.addToResourcePolicy(
      new iam.PolicyStatement({
        actions: ['s3:GetObject'],
        resources: [siteBucket.arnForObjects('*')],
        principals: [
          new iam.CanonicalUserPrincipal(
            oai.cloudFrontOriginAccessIdentityS3CanonicalUserId
          ),
        ],
      })
    );

    const distribution = new cloudfront.Distribution(this, 'Distribution', {
      defaultBehavior: {
        origin: new origins.S3Origin(siteBucket, {
          originAccessIdentity: oai,
        }),
        viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        allowedMethods: cloudfront.AllowedMethods.ALLOW_GET_HEAD,
        cachedMethods: cloudfront.CachedMethods.CACHE_GET_HEAD,
        compress: true,
        cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
      },
      domainNames: [domainName, wwwDomainName],
      certificate,
      minimumProtocolVersion: cloudfront.SecurityPolicyProtocol.TLS_V1_2_2021,
      errorResponses: [
        {
          httpStatus: 404,
          responseHttpStatus: 404,
          responsePagePath: '/index.html',
          ttl: cdk.Duration.minutes(5),
        },
        {
          httpStatus: 403,
          responseHttpStatus: 200,
          responsePagePath: '/index.html',
          ttl: cdk.Duration.minutes(5),
        },
      ],
      defaultRootObject: 'index.html',
      priceClass: cloudfront.PriceClass.PRICE_CLASS_100,
      enableIpv6: true,
      comment: `CloudFront distribution for ${domainName}`,
    });

    new s3deploy.BucketDeployment(this, 'DeployWebsite', {
      sources: [s3deploy.Source.asset(path.join(__dirname, '../static'))],
      destinationBucket: siteBucket,
      distribution,
      distributionPaths: ['/*'],
    });

    for (const [idSuffix, recordName] of [
      ['Apex', domainName],
      ['Www', wwwDomainName],
    ] as const) {
      new route53.ARecord(this, `${idSuffix}ARecord`, {
        zone: hostedZone,
        recordName,
        target: route53.RecordTarget.fromAlias(
          new targets.CloudFrontTarget(distribution)
        ),
      });
      new route53.AaaaRecord(this, `${idSuffix}AaaaRecord`, {
        zone: hostedZone,
        recordName,
        target: route53.RecordTarget.fromAlias(
          new targets.CloudFrontTarget(distribution)
        ),
      });
    }

    new cdk.CfnOutput(this, 'BucketName', {
      value: siteBucket.bucketName,
      description: 'S3 bucket for modality.network content',
    });
    new cdk.CfnOutput(this, 'DistributionId', {
      value: distribution.distributionId,
      description: 'CloudFront distribution ID for modality.network',
    });
    new cdk.CfnOutput(this, 'DistributionDomainName', {
      value: distribution.distributionDomainName,
      description: 'CloudFront domain name for modality.network',
    });
    new cdk.CfnOutput(this, 'Url', {
      value: `https://${domainName}`,
      description: 'URL for modality.network',
    });
    new cdk.CfnOutput(this, 'WwwUrl', {
      value: `https://${wwwDomainName}`,
      description: 'URL for www.modality.network',
    });
    new cdk.CfnOutput(this, 'CertificateArn', {
      value: certificate.certificateArn,
      description: 'ACM certificate ARN',
    });
  }
}
