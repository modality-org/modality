#!/bin/bash

# Deploy script for modal.money infrastructure
# This script helps deploy the AWS CDK infrastructure for modal.money

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "================================================"
echo "Modal.money Infrastructure Deployment"
echo "================================================"
echo ""

# Check if Node.js is installed
if ! command -v node &> /dev/null; then
    echo "❌ Error: Node.js is not installed"
    echo "Please install Node.js 18.x or later from https://nodejs.org/"
    exit 1
fi

echo "✅ Node.js version: $(node --version)"

# Check if AWS CLI is installed
if ! command -v aws &> /dev/null; then
    echo "❌ Error: AWS CLI is not installed"
    echo "Please install AWS CLI from https://aws.amazon.com/cli/"
    exit 1
fi

echo "✅ AWS CLI version: $(aws --version)"

# Check AWS credentials
if ! aws sts get-caller-identity &> /dev/null; then
    echo "❌ Error: AWS credentials not configured"
    echo "Please run: aws configure"
    exit 1
fi

AWS_ACCOUNT=$(aws sts get-caller-identity --query Account --output text)
echo "✅ AWS Account: $AWS_ACCOUNT"
echo ""

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo "📦 Installing dependencies..."
    npm install
    echo ""
fi

# Check if CDK is bootstrapped
echo "🔍 Checking CDK bootstrap status..."
if ! aws cloudformation describe-stacks --stack-name CDKToolkit --region us-east-1 &> /dev/null; then
    echo "⚠️  CDK is not bootstrapped in us-east-1"
    echo "📦 Bootstrapping CDK..."
    npx cdk bootstrap aws://$AWS_ACCOUNT/us-east-1
    echo ""
else
    echo "✅ CDK is already bootstrapped"
    echo ""
fi

# Synthesize the stack
echo "🔨 Synthesizing CloudFormation template..."
npm run synth
echo ""

# Show the diff
echo "📋 Reviewing changes..."
npm run diff || true
echo ""

# Prompt for confirmation
read -p "🚀 Ready to deploy? This will create AWS resources. Continue? (y/N) " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "❌ Deployment cancelled"
    exit 1
fi

# Deploy
echo ""
echo "🚀 Deploying infrastructure..."
echo "⏱️  This may take 20-40 minutes for initial deployment..."
echo ""
npm run deploy

echo ""
echo "================================================"
echo "✅ Deployment Complete!"
echo "================================================"
echo ""
echo "Your website should be accessible at:"
echo "  - https://www.modal.money"
echo "  - https://modal.money (redirects to www)"
echo ""
echo "Note: DNS propagation may take up to 5-15 minutes."
echo "      CloudFront distribution may take up to 30 minutes to fully deploy."
echo ""
echo "To update website content:"
echo "  1. Modify files in ../modal.money/"
echo "  2. Run: npm run deploy"
echo ""
echo "To destroy infrastructure:"
echo "  Run: npm run destroy"
echo ""

