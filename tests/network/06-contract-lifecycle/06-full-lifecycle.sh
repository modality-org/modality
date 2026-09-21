#!/usr/bin/env bash
# Complete contract lifecycle demonstration
# Creates, commits, pushes, and verifies a contract

set -e
cd "$(dirname "$0")"

echo "🔄 Complete Contract Lifecycle Demo"
echo "===================================="
echo ""

# Clean up any previous state
echo "Cleaning up previous state..."
rm -rf ./tmp
mkdir -p ./tmp/test-logs

# Step 1: Create contract
echo ""
echo "Step 1: Creating contract..."
echo "----------------------------"
./01-create-contract.sh

# Step 2: Add commits
echo ""
echo "Step 2: Adding commits..."
echo "-------------------------"
./02-commit-to-contract.sh

# Step 3: View status
echo ""
echo "Step 3: Viewing status..."
echo "-------------------------"
./03-view-status.sh

# Step 4: (Optional) Push to validators
echo ""
echo "Step 4: Pushing to validators..."
echo "---------------------------------"
echo "⚠️  Note: This requires a running validator. Skipping for now."
echo "    Run ./04-push-to-validators.sh manually if you have a validator running."

echo ""
echo "✅ Contract lifecycle demonstration complete!"
echo ""
echo "📋 Summary:"
echo "   ✓ Contract created"
echo "   ✓ Commits added (3)"
echo "   ✓ Status viewed"
echo ""
echo "📁 Contract location: ./tmp/my-contract"
echo ""
echo "🎯 Next steps:"
echo "   • View contract: cd ./tmp/my-contract && modal contract status"
echo "   • Add more commits: cd ./tmp/my-contract && modal contract commit --path /test.text --value 'data'"
echo "   • Push to network: ./04-push-to-validators.sh (requires validator)"

