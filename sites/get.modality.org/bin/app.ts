#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { GetModalityOrgStack } from '../lib/get-modality-org-stack';

const app = new cdk.App();

new GetModalityOrgStack(app, 'GetModalityOrgStack', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: 'us-east-1',
  },
  subdomainName: 'get.modality.org',
  description: 'Infrastructure for get.modality.org package distribution',
});

app.synth();
