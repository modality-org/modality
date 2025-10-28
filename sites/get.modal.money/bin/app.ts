#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { GetModalMoneyStack } from '../lib/get-modal-money-stack';

const app = new cdk.App();

// Deploy the get.modal.money infrastructure
// This stack creates:
// - S3 bucket for get.modal.money static content (Rust crate registry)
// - CloudFront distribution with HTTPS
// - SSL certificate
// - Route53 DNS record
new GetModalMoneyStack(app, 'GetModalMoneyStack', {
  env: {
    // Must be in us-east-1 for CloudFront SSL certificates
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: 'us-east-1',
  },
  subdomainName: 'get.modal.money',
  description: 'Infrastructure for get.modal.money static website (Rust crate registry)',
});

app.synth();

