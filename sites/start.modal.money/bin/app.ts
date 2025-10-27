#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { StartModalMoneyStack } from '../lib/start-modal-money-stack';

const app = new cdk.App();

// Deploy the start.modal.money infrastructure
// This stack creates:
// - S3 bucket for start.modal.money static content
// - CloudFront distribution with HTTPS
// - SSL certificate
// - Route53 DNS record
new StartModalMoneyStack(app, 'StartModalMoneyStack', {
  env: {
    // Must be in us-east-1 for CloudFront SSL certificates
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: 'us-east-1',
  },
  subdomainName: 'start.modal.money',
  description: 'Infrastructure for start.modal.money static website (installation page)',
});

app.synth();

