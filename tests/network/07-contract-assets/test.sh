#!/usr/bin/env bash
set -e

echo "================================================"
echo "Contract Assets Integration Test"
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

# Start the test
echo "Starting integration test..."
echo ""

# Set flag to skip network push in local mode
export SKIP_PUSH=1

# Step 0: Setup
run_step "Step 0: Setup" "./00-setup.sh" || exit 1

# Step 1: Create Alice's contract
run_step "Step 1: Create Alice's Contract" "./01-create-alice.sh" || exit 1
validate "Alice has .contract directory" "[ -d tmp/alice/.contract ]"
validate "Alice's config.json exists" "[ -f tmp/alice/.contract/config.json ]"

# Step 2: Create token asset
run_step "Step 2: Create Token Asset" "./02-create-token.sh" || exit 1

# Verify token was created
ALICE_CONTRACT_ID=$(cd tmp/alice && modal contract id)
validate "Alice's contract ID is not empty" "[ -n '$ALICE_CONTRACT_ID' ]"

# Step 3: Create Bob's contract
run_step "Step 3: Create Bob's Contract" "./03-create-bob.sh" || exit 1
validate "Bob has .contract directory" "[ -d tmp/bob/.contract ]"
validate "Bob's config.json exists" "[ -f tmp/bob/.contract/config.json ]"

# Step 4: Alice sends tokens
run_step "Step 4: Alice Sends Tokens" "./04-alice-sends-tokens.sh" || exit 1
validate "SEND commit ID was saved" "[ -f tmp/send-commit-id.txt ]"

# Step 5: Bob receives tokens
run_step "Step 5: Bob Receives Tokens" "./05-bob-receives-tokens.sh" || exit 1

# Step 6: Query balances
run_step "Step 6: Query Balances" "./06-query-balances.sh" || exit 1

# Step 7: Invalid double-send example
run_step "Step 7: Invalid Double-Send" "./08-invalid-double-send.sh" || exit 1

# Verify commit structure
echo ""
echo "Verifying commit structure..."

# Check Alice's CREATE commit
CREATE_COMMIT_ID=$(cat tmp/send-commit-id.txt | head -1)
# Get the previous commit (CREATE was before SEND)
# We'll validate commits exist, but not check specific structure since we don't save commit IDs

# Get commits from .contract directory
if [ -d "tmp/alice/.contract/commits" ]; then
    COMMIT_COUNT=$(ls tmp/alice/.contract/commits | wc -l | tr -d ' ')
    if [ "$COMMIT_COUNT" -ge 2 ]; then
        echo "${GREEN}✓ Alice has multiple commits (CREATE + SEND)${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo "${RED}✗ Alice should have at least 2 commits${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
else
    echo "${RED}✗ Alice's commits directory not found${NC}"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi

# Check Bob's commits
if [ -d "tmp/bob/.contract/commits" ]; then
    COMMIT_COUNT=$(ls tmp/bob/.contract/commits | wc -l | tr -d ' ')
    if [ "$COMMIT_COUNT" -ge 1 ]; then
        echo "${GREEN}✓ Bob has commits (RECV)${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo "${RED}✗ Bob should have at least 1 commit${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
else
    echo "${RED}✗ Bob's commits directory not found${NC}"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi

# Verify contract IDs are valid
ALICE_ID=$(cd tmp/alice && modal contract id)
BOB_ID=$(cd tmp/bob && modal contract id)

if [[ "$ALICE_ID" =~ ^12D3KooW ]]; then
    echo "${GREEN}✓ Alice's contract ID is valid format${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    echo "${RED}✗ Alice's contract ID format invalid${NC}"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi

if [[ "$BOB_ID" =~ ^12D3KooW ]]; then
    echo "${GREEN}✓ Bob's contract ID is valid format${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    echo "${RED}✗ Bob's contract ID format invalid${NC}"
    TESTS_FAILED=$((TESTS_FAILED + 1))
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
    echo "${GREEN}✅ All tests passed!${NC}"
    exit 0
else
    echo "${RED}❌ Some tests failed${NC}"
    exit 1
fi

