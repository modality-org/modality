#!/usr/bin/env bash
set -e

echo "================================================"
echo "Contract Assets Integration Test (with devnet1)"
echo "================================================"
echo ""

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track test results
TESTS_PASSED=0
TESTS_FAILED=0

# Helper function to run a test step
run_step() {
    local step_name=$1
    local script=$2
    
    echo ""
    echo "${YELLOW}Running: $step_name${NC}"
    
    if bash "$script"; then
        echo "${GREEN}✓ $step_name passed${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        return 0
    else
        echo "${RED}✗ $step_name failed${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi
}

# Helper function to validate output
validate() {
    local description=$1
    local condition=$2
    
    if eval "$condition"; then
        echo "${GREEN}✓ Validated: $description${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        return 0
    else
        echo "${RED}✗ Validation failed: $description${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi
}

# Cleanup function
cleanup() {
    echo ""
    echo "Cleaning up..."
    ./07-stop-validator.sh || true
}

# Set trap to cleanup on exit
trap cleanup EXIT

# Start the test
echo "Starting devnet1 integration test..."
echo ""

# Step 0: Setup
run_step "Step 0: Setup devnet1" "./00-setup-devnet1.sh" || exit 1

# Step 0.5: Start validator
run_step "Step 0.5: Start Validator" "./00b-start-validator.sh" || exit 1
validate "Validator is running on port 10101" "lsof -i :10101 -sTCP:LISTEN -t >/dev/null 2>&1"

# Give validator time to fully initialize consensus
echo ""
echo "⏳ Waiting for validator consensus to initialize..."
sleep 5
echo ""

# Step 1: Create Alice's contract
run_step "Step 1: Create Alice's Contract" "./01-create-alice.sh" || exit 1
validate "Alice's contract file exists" "[ -f data/alice/alice-contract.json ]"
validate "Alice has .contract directory" "[ -d data/alice/.contract ]"

# Step 2: Create token asset
run_step "Step 2: Create Token Asset" "./02-create-token.sh" || exit 1
validate "Token creation commit exists" "[ -f data/alice/create-token.json ]"

# Verify token was created
ALICE_CONTRACT_ID=$(cat data/alice/alice-contract.json | python3 -c "import sys, json; print(json.load(sys.stdin)['contract_id'])")
validate "Alice's contract ID is not empty" "[ -n '$ALICE_CONTRACT_ID' ]"

# Step 3: Create Bob's contract
run_step "Step 3: Create Bob's Contract" "./03-create-bob.sh" || exit 1
validate "Bob's contract file exists" "[ -f data/bob/bob-contract.json ]"
validate "Bob has .contract directory" "[ -d data/bob/.contract ]"

# Step 4: Alice sends tokens (with network push)
run_step "Step 4: Alice Sends Tokens" "./04-alice-sends-tokens.sh" || exit 1
validate "SEND commit file exists" "[ -f data/alice/send-tokens.json ]"
validate "SEND commit ID was saved" "[ -f data/send-commit-id.txt ]"

# Wait for network to process
echo ""
echo "⏳ Waiting for network to process commits..."
sleep 2

# Step 5: Bob receives tokens (with network push)
run_step "Step 5: Bob Receives Tokens" "./05-bob-receives-tokens.sh" || exit 1
validate "RECV commit file exists" "[ -f data/bob/recv-tokens.json ]"

# Wait for network to process
echo ""
echo "⏳ Waiting for network to process commits..."
sleep 2

# Step 6: Query balances
run_step "Step 6: Query Balances" "./06-query-balances.sh" || exit 1

# Check validator logs for asset processing
echo ""
echo "Checking validator logs for asset processing..."
if grep -q "Processed commit" tmp/test-logs/validator.log 2>/dev/null; then
    echo "${GREEN}✓ Validator processed commits${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    echo "${YELLOW}⚠ Could not verify commit processing in logs${NC}"
fi

# Print test summary
echo ""
echo "================================================"
echo "Test Summary"
echo "================================================"
echo ""
echo "${GREEN}Passed: $TESTS_PASSED${NC}"
echo "${RED}Failed: $TESTS_FAILED${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo "${GREEN}✅ All tests passed with devnet1!${NC}"
    exit 0
else
    echo "${RED}❌ Some tests failed${NC}"
    exit 1
fi

