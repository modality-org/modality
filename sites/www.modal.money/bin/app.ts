#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { ModalMoneyWebsiteStack } from '../lib/modal-money-website-stack';

const app = new cdk.App();

// Deploy the Modal.money website infrastructure
// This stack creates:
// - S3 buckets for www.modal.money and modal.money
// - CloudFront distributions for both domains
// - SSL certificates for HTTPS
// - Route53 records for DNS
// - modal.money redirects to www.modal.money
new ModalMoneyWebsiteStack(app, 'ModalMoneyWebsiteStack', {
  env: {
    // Must be in us-east-1 for CloudFront SSL certificates
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: 'us-east-1',
  },
  domainName: 'modal.money',
  subdomainName: 'www.modal.money',
  description: 'Infrastructure for modal.money static website with apex domain redirect',
});

app.synth();

