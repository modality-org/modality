#!/usr/bin/env bash
set -e

echo "================================================"
echo "Step 1: Create Alice's Contract"
echo "================================================"
echo ""

cd tmp/alice

echo "Creating Alice's contract..."
modal contract create

ALICE_CONTRACT_ID=$(modal contract id)

echo "✅ Alice's contract created!"
echo ""
echo "Contract ID: $ALICE_CONTRACT_ID"
echo ""
echo "Contract directory: tmp/alice/.contract/"
echo ""

cd ../..

