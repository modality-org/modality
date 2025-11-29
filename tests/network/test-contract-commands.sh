#!/bin/bash

# Test script for new contract commands
# Tests the local-first, git-like workflow

set -e

echo "🧪 Testing Contract Commands Rearchitecture"
echo "============================================="
echo

# Setup
TEST_DIR="/tmp/modal-contract-test-$$"
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

echo "📁 Test directory: $TEST_DIR"
echo

# Test 1: Create contract
echo "Test 1: Create contract"
echo "-----------------------"
modal contract create --output json > create_result.json
cat create_result.json | head -10
echo "✅ Contract created"
echo

# Extract contract ID
CONTRACT_ID=$(cat create_result.json | grep -o '"contract_id":"[^"]*"' | cut -d'"' -f4)
echo "   Contract ID: $CONTRACT_ID"
echo

# Verify directory structure
echo "Test 2: Verify directory structure"
echo "-----------------------------------"
if [ -d ".contract" ]; then
    echo "✅ .contract/ directory exists"
else
    echo "❌ .contract/ directory missing"
    exit 1
fi

if [ -f ".contract/config.json" ]; then
    echo "✅ config.json exists"
else
    echo "❌ config.json missing"
    exit 1
fi

if [ -f ".contract/genesis.json" ]; then
    echo "✅ genesis.json exists"
else
    echo "❌ genesis.json missing"
    exit 1
fi

if [ -f ".contract/HEAD" ]; then
    echo "✅ HEAD file exists"
else
    echo "❌ HEAD file missing"
    exit 1
fi

echo

# Test 3: Make a commit
echo "Test 3: Create commit"
echo "---------------------"
modal contract commit --path /data --value "hello world" --output json > commit1_result.json
cat commit1_result.json
echo "✅ Commit created"
echo

# Test 4: Make another commit
echo "Test 4: Create second commit"
echo "----------------------------"
modal contract commit --path /rate --value 7.5 --output json > commit2_result.json
cat commit2_result.json
echo "✅ Second commit created"
echo

# Test 5: Check status
echo "Test 5: Check status"
echo "--------------------"
modal contract status
echo "✅ Status displayed"
echo

# Test 6: List commits
echo "Test 6: Verify commits stored"
echo "------------------------------"
COMMIT_COUNT=$(ls -1 .contract/commits/*.json | wc -l | tr -d ' ')
echo "   Commits found: $COMMIT_COUNT"
if [ "$COMMIT_COUNT" -ge "2" ]; then
    echo "✅ Multiple commits stored"
else
    echo "❌ Expected at least 2 commits, found $COMMIT_COUNT"
    exit 1
fi
echo

# Test 7: Verify commit structure
echo "Test 7: Verify commit structure"
echo "--------------------------------"
FIRST_COMMIT=$(ls .contract/commits/*.json | head -1)
if command -v jq &> /dev/null; then
    echo "   First commit contents:"
    cat "$FIRST_COMMIT" | jq .
    echo "✅ Commit has valid JSON structure"
else
    echo "   (skipping JSON validation - jq not available)"
    cat "$FIRST_COMMIT"
fi
echo

# Test 8: JSON output formats
echo "Test 8: JSON output formats"
echo "---------------------------"
modal contract status --output json > status.json
if command -v jq &> /dev/null; then
    cat status.json | jq .
    echo "✅ Status JSON output valid"
else
    cat status.json
    echo "   (skipping JSON validation - jq not available)"
fi
echo

# Cleanup
echo "🧹 Cleaning up..."
cd /
rm -rf "$TEST_DIR"
echo "✅ Test directory removed"
echo

echo "============================================="
echo "✅ All tests passed!"
echo "============================================="

