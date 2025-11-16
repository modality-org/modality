#!/usr/bin/env bash
set -e

echo "================================================"
echo "Network Parameters Integration Test"
echo "================================================"
echo ""
echo "This test verifies that:"
echo "  1. Network genesis contracts are created with parameters"
echo "  2. POST actions are processed during consensus"
echo "  3. Parameters are readable from contract state"
echo "  4. Nodes can load parameters from genesis contract"
echo ""

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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
    echo "${BLUE}Cleaning up...${NC}"
    bash ./99-cleanup.sh 2>/dev/null || true
}

# Set trap to cleanup on exit
trap cleanup EXIT

# Start the test
echo "Starting integration test..."
echo ""

# Step 0: Setup and start devnet1
run_step "Step 0: Setup devnet1" "./00-setup.sh" || exit 1
validate "Node storage exists" "[ -d ./tmp/test-network-params ]"

# Step 1: Verify genesis contract exists
run_step "Step 1: Verify genesis contract in config" "./01-verify-genesis-contract.sh" || exit 1

# Step 2: Start the node
run_step "Step 2: Start node" "./02-start-node.sh" || exit 1
sleep 3  # Give node time to start

# Step 3: Inspect network parameters from datastore
run_step "Step 3: Query network parameters" "./03-query-parameters.sh" || exit 1

# Step 4: Verify parameters are loaded correctly
run_step "Step 4: Verify parameter values" "./04-verify-values.sh" || exit 1

# Step 5: Stop the node
run_step "Step 5: Stop node" "./05-stop-node.sh" || exit 1

# Print summary
echo ""
echo "================================================"
echo "Test Summary"
echo "================================================"
echo "${GREEN}Tests Passed: $TESTS_PASSED${NC}"
if [ $TESTS_FAILED -gt 0 ]; then
    echo "${RED}Tests Failed: $TESTS_FAILED${NC}"
    exit 1
else
    echo "${GREEN}All tests passed!${NC}"
    exit 0
fi

