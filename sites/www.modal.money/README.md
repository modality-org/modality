# Modal.money Website Infrastructure

AWS CDK infrastructure code for deploying the modal.money static website with CloudFront, S3, and Route53.

## Architecture

This CDK stack deploys:

- **www.modal.money**: Primary static website served from S3 via CloudFront
- **modal.money**: Apex domain that redirects to www.modal.money
- **CloudFront**: Global CDN for fast content delivery with HTTPS
- **Route53**: DNS management for both domains
- **ACM**: SSL/TLS certificates for HTTPS
- **S3**: Secure content storage with versioning

### Key Features

- ✅ HTTPS enabled with automatic certificate management
- ✅ IPv6 support
- ✅ Apex domain redirect (modal.money → www.modal.money)
- ✅ CloudFront caching and compression
- ✅ Secure S3 buckets (no public access)
- ✅ Automatic website deployment from `../modal.money/`
- ✅ Custom error pages

## Prerequisites

Before deploying, ensure you have:

1. **AWS Account**: Active AWS account with appropriate permissions
2. **AWS CLI**: Installed and configured with credentials
   ```bash
   aws configure
   ```
3. **Node.js**: Version 18.x or later
4. **Route53 Hosted Zone**: A hosted zone for `modal.money` must already exist in your AWS account

## Installation

1. Install dependencies:
   ```bash
   npm install
   ```

2. Bootstrap CDK (first time only):
   ```bash
   npx cdk bootstrap aws://ACCOUNT-NUMBER/us-east-1
   ```
   Replace `ACCOUNT-NUMBER` with your AWS account ID. The region must be `us-east-1` for CloudFront certificates.

## Deployment

### Quick Deploy

Deploy everything with:
```bash
npm run deploy
```

### Step-by-Step Deployment

1. **Synthesize CloudFormation template** (optional, to review):
   ```bash
   npm run synth
   ```

2. **View changes before deploying**:
   ```bash
   npm run diff
   ```

3. **Deploy the stack**:
   ```bash
   npm run deploy
   ```

4. **Accept changes**: CDK will prompt you to review IAM changes and security group modifications. Type `y` to proceed.

### Deployment Notes

- **Initial deployment** takes 20-40 minutes due to CloudFront distribution creation and DNS propagation
- **Certificate validation** requires DNS records to be created automatically (typically 5-10 minutes)
- **CloudFront propagation** to all edge locations takes 15-30 minutes
- **Subsequent deployments** are much faster (5-10 minutes)

## Post-Deployment

After deployment, you'll see outputs including:

- S3 bucket names
- CloudFront distribution IDs
- Website URLs
- Certificate ARN

### Verify Deployment

1. Check certificate validation:
   ```bash
   aws acm describe-certificate --certificate-arn <CERTIFICATE_ARN>
   ```

2. Test the websites:
   ```bash
   curl -I https://www.modal.money
   curl -I https://modal.money  # Should redirect to www
   ```

3. View CloudFront distributions:
   ```bash
   aws cloudfront list-distributions
   ```

## Updating Website Content

To update the website content:

1. Modify files in `../modal.money/`
2. Run deployment again:
   ```bash
   npm run deploy
   ```
3. Content will be automatically uploaded to S3 and CloudFront cache will be invalidated

### Manual Content Upload

If you prefer to upload content manually:

```bash
# Upload to S3
aws s3 sync ../modal.money/ s3://www.modal.money-content/ --delete

# Invalidate CloudFront cache
aws cloudfront create-invalidation \
  --distribution-id <DISTRIBUTION_ID> \
  --paths "/*"
```

## Managing the Infrastructure

### View Stack Details

```bash
npx cdk ls
```

### View Stack Differences

```bash
npm run diff
```

### Destroy Infrastructure

**⚠️ Warning**: This will delete all resources, including S3 buckets. Ensure you have backups!

```bash
npm run destroy
```

## Cost Considerations

Estimated monthly costs (assuming moderate traffic):

- **CloudFront**: ~$1-10/month (depends on data transfer)
- **S3**: ~$0.50-2/month (storage and requests)
- **Route53**: ~$1/month (hosted zone + queries)
- **ACM Certificate**: Free
- **Total**: ~$2.50-15/month

Costs scale with traffic. CloudFront has a generous free tier for the first year.

## Troubleshooting

### Certificate Validation Stuck

If certificate validation takes too long:
1. Check Route53 for CNAME validation records
2. Ensure your domain's nameservers point to AWS Route53
3. Wait up to 30 minutes for DNS propagation

### CloudFront 403 Errors

If you get 403 errors:
1. Check S3 bucket policy allows CloudFront OAI
2. Verify files exist in S3: `aws s3 ls s3://www.modal.money-content/`
3. Check CloudFront distribution origin settings

### DNS Not Resolving

1. Verify Route53 records exist: `dig www.modal.money`
2. Check nameservers: `dig NS modal.money`
3. Wait for DNS propagation (up to 48 hours, usually 5-15 minutes)

### Deploy Fails with "Bucket Already Exists"

S3 bucket names are globally unique. If deployment fails:
1. Check if buckets already exist in your account
2. Manually delete them if they're orphaned
3. Or modify bucket names in `lib/modal-money-website-stack.ts`

## File Structure

```
www.modal.money/
├── bin/
│   └── app.ts                 # CDK app entry point
├── lib/
│   └── modal-money-website-stack.ts  # Main stack definition
├── package.json               # Dependencies and scripts
├── tsconfig.json              # TypeScript configuration
├── cdk.json                   # CDK configuration
└── README.md                  # This file
```

## Security

- All S3 buckets have public access blocked
- CloudFront uses Origin Access Identity (OAI) for S3 access
- SSL/TLS enforced for all connections
- Bucket policies enforce HTTPS
- S3 versioning enabled for content bucket
- Server-side encryption enabled

## Support

For issues or questions:
1. Check CloudWatch Logs for Lambda@Edge/CloudFront Functions
2. Review CloudFormation events in AWS Console
3. Check CDK documentation: https://docs.aws.amazon.com/cdk/

## License

MIT

