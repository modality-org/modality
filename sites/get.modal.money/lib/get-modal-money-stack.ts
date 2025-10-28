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

export interface GetModalMoneyStackProps extends cdk.StackProps {
  subdomainName: string;
}

export class GetModalMoneyStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props: GetModalMoneyStackProps) {
    super(scope, id, props);

    const { subdomainName } = props;
    const domainName = 'modal.money';

    // Look up the existing hosted zone for modal.money
    // Note: This assumes you already have a Route53 hosted zone for modal.money
    const hostedZone = route53.HostedZone.fromLookup(this, 'HostedZone', {
      domainName: domainName,
    });

    // Create SSL certificate for get.modal.money
    const certificate = new acm.Certificate(this, 'Certificate', {
      domainName: subdomainName,
      validation: acm.CertificateValidation.fromDns(hostedZone),
    });

    // ========================================
    // S3 Bucket for Static Content
    // ========================================

    // S3 bucket for get.modal.money content (Rust crate registry)
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

    // Origin Access Identity for CloudFront to access S3
    const oai = new cloudfront.OriginAccessIdentity(this, 'OAI', {
      comment: `OAI for ${subdomainName}`,
    });

    // Grant CloudFront access to the bucket
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

    // ========================================
    // CloudFront Distribution
    // ========================================

    // CloudFront distribution for get.modal.money
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
      certificate: certificate,
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
          responseHttpStatus: 403,
          responsePagePath: '/index.html',
          ttl: cdk.Duration.minutes(5),
        },
      ],
      defaultRootObject: 'index.html',
      priceClass: cloudfront.PriceClass.PRICE_CLASS_100, // Use only North America and Europe
      enableIpv6: true,
      comment: `CloudFront distribution for ${subdomainName} (Rust crate registry)`,
    });

    // ========================================
    // Deploy Static Content
    // ========================================

    // Deploy website content from registry/ directory to S3
    new s3deploy.BucketDeployment(this, 'DeployRegistry', {
      sources: [s3deploy.Source.asset(path.join(__dirname, '../registry'))],
      destinationBucket: contentBucket,
      distribution: distribution,
      distributionPaths: ['/*'],
    });

    // ========================================
    // Route53 DNS Records
    // ========================================

    // Route53 A record for get.modal.money
    new route53.ARecord(this, 'ARecord', {
      zone: hostedZone,
      recordName: subdomainName,
      target: route53.RecordTarget.fromAlias(
        new targets.CloudFrontTarget(distribution)
      ),
    });

    // Route53 AAAA record for get.modal.money (IPv6)
    new route53.AaaaRecord(this, 'AaaaRecord', {
      zone: hostedZone,
      recordName: subdomainName,
      target: route53.RecordTarget.fromAlias(
        new targets.CloudFrontTarget(distribution)
      ),
    });

    // ========================================
    // Outputs
    // ========================================

    new cdk.CfnOutput(this, 'BucketName', {
      value: contentBucket.bucketName,
      description: 'S3 bucket for get.modal.money content',
      exportName: 'GetModalMoneyBucketName',
    });

    new cdk.CfnOutput(this, 'DistributionId', {
      value: distribution.distributionId,
      description: 'CloudFront distribution ID for get.modal.money',
      exportName: 'GetModalMoneyDistributionId',
    });

    new cdk.CfnOutput(this, 'DistributionDomainName', {
      value: distribution.distributionDomainName,
      description: 'CloudFront domain name for get.modal.money',
    });

    new cdk.CfnOutput(this, 'Url', {
      value: `https://${subdomainName}`,
      description: 'URL for get.modal.money',
    });

    new cdk.CfnOutput(this, 'CertificateArn', {
      value: certificate.certificateArn,
      description: 'ACM certificate ARN',
      exportName: 'GetModalMoneyCertificateArn',
    });
  }
}

