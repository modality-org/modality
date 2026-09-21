import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as origins from 'aws-cdk-lib/aws-cloudfront-origins';
import * as route53 from 'aws-cdk-lib/aws-route53';
import * as targets from 'aws-cdk-lib/aws-route53-targets';
import * as acm from 'aws-cdk-lib/aws-certificatemanager';
import * as iam from 'aws-cdk-lib/aws-iam';

export interface GetModalityOrgStackProps extends cdk.StackProps {
  subdomainName: string;
}

export class GetModalityOrgStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props: GetModalityOrgStackProps) {
    super(scope, id, props);

    const { subdomainName } = props;
    const domainName = 'modality.org';

    const hostedZone = route53.HostedZone.fromLookup(this, 'HostedZone', {
      domainName,
    });

    const certificate = new acm.Certificate(this, 'Certificate', {
      domainName: subdomainName,
      validation: acm.CertificateValidation.fromDns(hostedZone),
    });

    const contentBucket = new s3.Bucket(this, 'ContentBucket', {
      bucketName: `${subdomainName}-content`,
      publicReadAccess: false,
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      autoDeleteObjects: false,
      versioned: true,
      encryption: s3.BucketEncryption.S3_MANAGED,
      enforceSSL: true,
    });

    const oai = new cloudfront.OriginAccessIdentity(this, 'OAI', {
      comment: `OAI for ${subdomainName}`,
    });

    contentBucket.addToResourcePolicy(
      new iam.PolicyStatement({
        actions: ['s3:GetObject'],
        resources: [contentBucket.arnForObjects('*')],
        principals: [
          new iam.CanonicalUserPrincipal(
            oai.cloudFrontOriginAccessIdentityS3CanonicalUserId
          ),
        ],
      })
    );

    const distribution = new cloudfront.Distribution(this, 'Distribution', {
      defaultBehavior: {
        origin: origins.S3BucketOrigin.withOriginAccessIdentity(contentBucket, {
          originAccessIdentity: oai,
        }),
        viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        allowedMethods: cloudfront.AllowedMethods.ALLOW_GET_HEAD,
        cachedMethods: cloudfront.CachedMethods.CACHE_GET_HEAD,
        compress: true,
        cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
      },
      domainNames: [subdomainName],
      certificate,
      minimumProtocolVersion: cloudfront.SecurityPolicyProtocol.TLS_V1_2_2021,
      defaultRootObject: 'index.html',
      priceClass: cloudfront.PriceClass.PRICE_CLASS_100,
      enableIpv6: true,
      comment: `CloudFront distribution for ${subdomainName}`,
    });

    new route53.ARecord(this, 'ARecord', {
      zone: hostedZone,
      recordName: subdomainName,
      target: route53.RecordTarget.fromAlias(
        new targets.CloudFrontTarget(distribution)
      ),
    });

    new route53.AaaaRecord(this, 'AaaaRecord', {
      zone: hostedZone,
      recordName: subdomainName,
      target: route53.RecordTarget.fromAlias(
        new targets.CloudFrontTarget(distribution)
      ),
    });

    new cdk.CfnOutput(this, 'BucketName', {
      value: contentBucket.bucketName,
      description: 'S3 bucket for get.modality.org content',
      exportName: 'GetModalityOrgBucketName',
    });

    new cdk.CfnOutput(this, 'DistributionId', {
      value: distribution.distributionId,
      description: 'CloudFront distribution ID for get.modality.org',
      exportName: 'GetModalityOrgDistributionId',
    });

    new cdk.CfnOutput(this, 'DistributionDomainName', {
      value: distribution.distributionDomainName,
      description: 'CloudFront domain name for get.modality.org',
    });

    new cdk.CfnOutput(this, 'Url', {
      value: `https://${subdomainName}`,
      description: 'URL for get.modality.org',
    });

    new cdk.CfnOutput(this, 'CertificateArn', {
      value: certificate.certificateArn,
      description: 'ACM certificate ARN',
      exportName: 'GetModalityOrgCertificateArn',
    });
  }
}
