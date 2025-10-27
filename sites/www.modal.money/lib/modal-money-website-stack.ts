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

export interface ModalMoneyWebsiteStackProps extends cdk.StackProps {
  domainName: string;
  subdomainName: string;
}

export class ModalMoneyWebsiteStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props: ModalMoneyWebsiteStackProps) {
    super(scope, id, props);

    const { domainName, subdomainName } = props;

    // Look up the existing hosted zone for modal.money
    // Note: This assumes you already have a Route53 hosted zone for modal.money
    const hostedZone = route53.HostedZone.fromLookup(this, 'HostedZone', {
      domainName: domainName,
    });

    // Create SSL certificate for both domains
    const certificate = new acm.Certificate(this, 'Certificate', {
      domainName: subdomainName,
      subjectAlternativeNames: [domainName],
      validation: acm.CertificateValidation.fromDns(hostedZone),
    });

    // ========================================
    // WWW Subdomain - Main Static Website
    // ========================================

    // S3 bucket for www.modal.money content
    const wwwBucket = new s3.Bucket(this, 'WwwBucket', {
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
    const wwwOai = new cloudfront.OriginAccessIdentity(this, 'WwwOAI', {
      comment: `OAI for ${subdomainName}`,
    });

    // Grant CloudFront access to the bucket
    wwwBucket.addToResourcePolicy(
      new iam.PolicyStatement({
        actions: ['s3:GetObject'],
        resources: [wwwBucket.arnForObjects('*')],
        principals: [
          new iam.CanonicalUserPrincipal(
            wwwOai.cloudFrontOriginAccessIdentityS3CanonicalUserId
          ),
        ],
      })
    );

    // CloudFront distribution for www.modal.money
    const wwwDistribution = new cloudfront.Distribution(this, 'WwwDistribution', {
      defaultBehavior: {
        origin: new origins.S3Origin(wwwBucket, {
          originAccessIdentity: wwwOai,
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
      comment: `CloudFront distribution for ${subdomainName}`,
    });

    // Deploy website content from ../modal.money to www S3 bucket
    new s3deploy.BucketDeployment(this, 'DeployWebsite', {
      sources: [s3deploy.Source.asset(path.join(__dirname, '../../modal.money'))],
      destinationBucket: wwwBucket,
      distribution: wwwDistribution,
      distributionPaths: ['/*'],
    });

    // Route53 A record for www.modal.money
    new route53.ARecord(this, 'WwwARecord', {
      zone: hostedZone,
      recordName: subdomainName,
      target: route53.RecordTarget.fromAlias(
        new targets.CloudFrontTarget(wwwDistribution)
      ),
    });

    // Route53 AAAA record for www.modal.money (IPv6)
    new route53.AaaaRecord(this, 'WwwAaaaRecord', {
      zone: hostedZone,
      recordName: subdomainName,
      target: route53.RecordTarget.fromAlias(
        new targets.CloudFrontTarget(wwwDistribution)
      ),
    });

    // ========================================
    // Apex Domain - Redirect to WWW
    // ========================================

    // S3 bucket for apex domain redirect
    const apexBucket = new s3.Bucket(this, 'ApexBucket', {
      bucketName: `${domainName}-redirect`,
      publicReadAccess: false,
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      autoDeleteObjects: false,
      websiteRedirect: {
        hostName: subdomainName,
        protocol: s3.RedirectProtocol.HTTPS,
      },
      encryption: s3.BucketEncryption.S3_MANAGED,
      enforceSSL: true,
    });

    // Origin Access Identity for apex CloudFront
    const apexOai = new cloudfront.OriginAccessIdentity(this, 'ApexOAI', {
      comment: `OAI for ${domainName}`,
    });

    // Grant CloudFront access to the apex bucket
    apexBucket.addToResourcePolicy(
      new iam.PolicyStatement({
        actions: ['s3:GetObject'],
        resources: [apexBucket.arnForObjects('*')],
        principals: [
          new iam.CanonicalUserPrincipal(
            apexOai.cloudFrontOriginAccessIdentityS3CanonicalUserId
          ),
        ],
      })
    );

    // CloudFront Function for redirect
    const redirectFunction = new cloudfront.Function(this, 'RedirectFunction', {
      code: cloudfront.FunctionCode.fromInline(`
function handler(event) {
  var request = event.request;
  var response = {
    statusCode: 301,
    statusDescription: 'Moved Permanently',
    headers: {
      'location': { value: 'https://${subdomainName}' + request.uri }
    }
  };
  return response;
}
      `),
      comment: `Redirect ${domainName} to ${subdomainName}`,
    });

    // CloudFront distribution for apex domain (modal.money)
    const apexDistribution = new cloudfront.Distribution(this, 'ApexDistribution', {
      defaultBehavior: {
        origin: new origins.S3Origin(apexBucket, {
          originAccessIdentity: apexOai,
        }),
        viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        allowedMethods: cloudfront.AllowedMethods.ALLOW_GET_HEAD,
        cachedMethods: cloudfront.CachedMethods.CACHE_GET_HEAD,
        functionAssociations: [
          {
            function: redirectFunction,
            eventType: cloudfront.FunctionEventType.VIEWER_REQUEST,
          },
        ],
      },
      domainNames: [domainName],
      certificate: certificate,
      minimumProtocolVersion: cloudfront.SecurityPolicyProtocol.TLS_V1_2_2021,
      priceClass: cloudfront.PriceClass.PRICE_CLASS_100,
      enableIpv6: true,
      comment: `CloudFront distribution for ${domainName} (redirect)`,
    });

    // Route53 A record for apex domain
    new route53.ARecord(this, 'ApexARecord', {
      zone: hostedZone,
      recordName: domainName,
      target: route53.RecordTarget.fromAlias(
        new targets.CloudFrontTarget(apexDistribution)
      ),
    });

    // Route53 AAAA record for apex domain (IPv6)
    new route53.AaaaRecord(this, 'ApexAaaaRecord', {
      zone: hostedZone,
      recordName: domainName,
      target: route53.RecordTarget.fromAlias(
        new targets.CloudFrontTarget(apexDistribution)
      ),
    });

    // ========================================
    // Outputs
    // ========================================

    new cdk.CfnOutput(this, 'WwwBucketName', {
      value: wwwBucket.bucketName,
      description: 'S3 bucket for www.modal.money content',
      exportName: 'ModalMoneyWwwBucketName',
    });

    new cdk.CfnOutput(this, 'WwwDistributionId', {
      value: wwwDistribution.distributionId,
      description: 'CloudFront distribution ID for www.modal.money',
      exportName: 'ModalMoneyWwwDistributionId',
    });

    new cdk.CfnOutput(this, 'WwwDistributionDomainName', {
      value: wwwDistribution.distributionDomainName,
      description: 'CloudFront domain name for www.modal.money',
    });

    new cdk.CfnOutput(this, 'WwwUrl', {
      value: `https://${subdomainName}`,
      description: 'URL for www.modal.money',
    });

    new cdk.CfnOutput(this, 'ApexBucketName', {
      value: apexBucket.bucketName,
      description: 'S3 bucket for modal.money redirect',
      exportName: 'ModalMoneyApexBucketName',
    });

    new cdk.CfnOutput(this, 'ApexDistributionId', {
      value: apexDistribution.distributionId,
      description: 'CloudFront distribution ID for modal.money',
      exportName: 'ModalMoneyApexDistributionId',
    });

    new cdk.CfnOutput(this, 'ApexUrl', {
      value: `https://${domainName}`,
      description: 'URL for modal.money (redirects to www)',
    });

    new cdk.CfnOutput(this, 'CertificateArn', {
      value: certificate.certificateArn,
      description: 'ACM certificate ARN',
      exportName: 'ModalMoneyCertificateArn',
    });
  }
}

