#!/usr/bin/env bash
# Add commits to a local contract
# Demonstrates creating multiple commits with different data types

set -e
cd "$(dirname "$0")"

CONTRACT_DIR="./tmp/my-contract"

echo "📝 Adding commits to contract..."
echo "================================="
echo ""

# Ensure contract exists
if [ ! -d "$CONTRACT_DIR/.contract" ]; then
    echo "❌ Error: Contract not found. Run ./01-create-contract.sh first."
    exit 1
fi

cd "$CONTRACT_DIR"

# Commit 1: String data
echo "Creating commit 1: String value..."
modal contract commit --path "/data/message" --value "Hello, Modality!" --output json | tee commit1.json
COMMIT1_ID=$(cat commit1.json | grep -o '"commit_id":"[^"]*"' | cut -d'"' -f4 || echo "")
echo "✅ Commit 1 created: $COMMIT1_ID"
echo ""

# Commit 2: Numeric data
echo "Creating commit 2: Numeric value..."
modal contract commit --path "/config/rate" --value 7.5 --output json | tee commit2.json
COMMIT2_ID=$(cat commit2.json | grep -o '"commit_id":"[^"]*"' | cut -d'"' -f4 || echo "")
echo "✅ Commit 2 created: $COMMIT2_ID"
echo ""

# Commit 3: Another string
echo "Creating commit 3: Additional data..."
modal contract commit --path "/data/status" --value "active" --output json | tee commit3.json
COMMIT3_ID=$(cat commit3.json | grep -o '"commit_id":"[^"]*"' | cut -d'"' -f4 || echo "")
echo "✅ Commit 3 created: $COMMIT3_ID"
echo ""

echo "✅ All commits created successfully!"
echo ""
echo "📊 Commit Summary:"
echo "   Total commits: 3"
echo "   Commit 1: $COMMIT1_ID"
echo "   Commit 2: $COMMIT2_ID"
echo "   Commit 3: $COMMIT3_ID"
echo ""
echo "📁 Commits stored in: $CONTRACT_DIR/.contract/commits/"
ls -la .contract/commits/
echo ""
echo "💡 Tip: View contract status with: modal contract status"

