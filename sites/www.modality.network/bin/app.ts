#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { ModalityNetworkWebsiteStack } from '../lib/modality-network-website-stack';

const app = new cdk.App();

new ModalityNetworkWebsiteStack(app, 'ModalityNetworkWebsiteStack', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: 'us-east-1',
  },
  domainName: 'modality.network',
  wwwDomainName: 'www.modality.network',
  description: 'Infrastructure for modality.network static website',
});

app.synth();
