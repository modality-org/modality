#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 3: Create Bob's Contract"
echo "================================================"
echo ""

cd tmp/bob

echo "Creating Bob's contract..."
modal contract create

BOB_CONTRACT_ID=$(modal contract id)

echo "✅ Bob's contract created!"
echo ""
echo "Contract ID: $BOB_CONTRACT_ID"
echo ""
echo "Contract directory: tmp/bob/.contract/"
echo ""

cd ../..

