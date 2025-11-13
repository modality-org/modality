#!/usr/bin/env bash
set -e

echo "================================================"
echo "Example: Invalid Double-Send (Insufficient Balance)"
echo "================================================"
echo ""
echo "This example demonstrates that validators reject"
echo "SEND commits when the sender lacks sufficient balance."
echo ""

cd data/alice

echo "📊 Current State:"
echo ""

# Check Alice's current balance
echo "Alice's current balance:"
modal contract assets balance --asset-id my_token
echo ""

# Try to send more than Alice has
echo "❌ Attempting to send 1,500,000 tokens (Alice only has ~990,000)..."
echo ""

# This will fail validation at consensus level
set +e  # Don't exit on error
RESULT=$(modal contract commit \
  --method send \
  --asset-id my_token \
  --to-contract 12D3KooWBob \
  --amount 1500000 2>&1)
EXIT_CODE=$?
set -e

if [ $EXIT_CODE -eq 0 ]; then
    echo "⚠️  WARNING: SEND was accepted locally!"
    echo "   (It will be rejected by validators during consensus)"
    COMMIT_ID=$(echo "$RESULT" | grep "Commit ID:" | awk '{print $3}')
    echo ""
    echo "   Commit ID: $COMMIT_ID"
    echo ""
    echo "   Let's try to push it..."
    
    # Try to push - this is where validation happens with network
    if [ -n "$SKIP_PUSH" ]; then
        echo ""
        echo "   ⚠️  Skipping push (local mode)"
        echo "   In network mode, the validator would reject this commit"
        echo "   with error: 'Insufficient balance: have 990000, need 1500000'"
    else
        set +e
        PUSH_RESULT=$(modal contract push \
          --remote /ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd \
          --remote-name origin 2>&1)
        PUSH_CODE=$?
        set -e
        
        if [ $PUSH_CODE -ne 0 ]; then
            echo ""
            echo "   ✅ VALIDATOR REJECTED THE COMMIT!"
            echo "   Error: $PUSH_RESULT"
        else
            echo ""
            echo "   ❌ ERROR: Push succeeded when it should have failed"
            echo "   This indicates a validation bug"
        fi
    fi
else
    echo "✅ Local validation caught the error:"
    echo ""
    echo "$RESULT"
fi

echo ""
echo "================================================"
echo "Key Points:"
echo "================================================"
echo ""
echo "1. Local validation may allow the commit to be created"
echo "2. Validators enforce balance checks at consensus level"
echo "3. Invalid SEND commits are rejected with clear errors"
echo "4. Balance protection prevents double-spending"
echo ""
echo "This ensures that assets cannot be created from nothing"
echo "and maintains the integrity of the asset system."
echo ""

cd ../..

