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

# Step 4: Alice sends tokens
run_step "Step 4: Alice Sends Tokens" "./04-alice-sends-tokens.sh" || exit 1
validate "SEND commit file exists" "[ -f data/alice/send-tokens.json ]"
validate "SEND commit ID was saved" "[ -f data/send-commit-id.txt ]"

# Step 5: Bob receives tokens
run_step "Step 5: Bob Receives Tokens" "./05-bob-receives-tokens.sh" || exit 1
validate "RECV commit file exists" "[ -f data/bob/recv-tokens.json ]"

# Step 6: Query balances
run_step "Step 6: Query Balances" "./06-query-balances.sh" || exit 1

# Verify commit structure
echo ""
echo "Verifying commit structure..."

# Check Alice's CREATE commit
CREATE_COMMIT_ID=$(cat data/alice/create-token.json | python3 -c "import sys, json; print(json.load(sys.stdin)['commit_id'])")
if [ -f "data/alice/.contract/commits/${CREATE_COMMIT_ID}.json" ]; then
    echo "${GREEN}✓ CREATE commit file exists${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
    
    # Check structure using python for nested JSON
    python3 -c "
import json, sys
with open('data/alice/.contract/commits/${CREATE_COMMIT_ID}.json') as f:
    commit = json.load(f)
    if commit.get('body') and len(commit['body']) > 0:
        action = commit['body'][0]
        if action.get('method') == 'create':
            print('${GREEN}✓ CREATE commit has correct method${NC}')
            sys.exit(0)
sys.exit(1)
" && TESTS_PASSED=$((TESTS_PASSED + 1)) || TESTS_FAILED=$((TESTS_FAILED + 1))
    
    python3 -c "
import json, sys
with open('data/alice/.contract/commits/${CREATE_COMMIT_ID}.json') as f:
    commit = json.load(f)
    if commit.get('body') and len(commit['body']) > 0:
        action = commit['body'][0]
        if action.get('value', {}).get('asset_id') == 'my_token':
            print('${GREEN}✓ CREATE commit has correct asset_id${NC}')
            sys.exit(0)
sys.exit(1)
" && TESTS_PASSED=$((TESTS_PASSED + 1)) || TESTS_FAILED=$((TESTS_FAILED + 1))
    
    python3 -c "
import json, sys
with open('data/alice/.contract/commits/${CREATE_COMMIT_ID}.json') as f:
    commit = json.load(f)
    if commit.get('body') and len(commit['body']) > 0:
        action = commit['body'][0]
        if action.get('value', {}).get('quantity') == 1000000:
            print('${GREEN}✓ CREATE commit has correct quantity${NC}')
            sys.exit(0)
sys.exit(1)
" && TESTS_PASSED=$((TESTS_PASSED + 1)) || TESTS_FAILED=$((TESTS_FAILED + 1))
else
    echo "${RED}✗ CREATE commit file not found${NC}"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi

# Check Alice's SEND commit
SEND_COMMIT_ID=$(cat data/alice/send-tokens.json | python3 -c "import sys, json; print(json.load(sys.stdin)['commit_id'])")
if [ -f "data/alice/.contract/commits/${SEND_COMMIT_ID}.json" ]; then
    echo "${GREEN}✓ SEND commit file exists${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
    
    python3 -c "
import json, sys
with open('data/alice/.contract/commits/${SEND_COMMIT_ID}.json') as f:
    commit = json.load(f)
    if commit.get('body') and len(commit['body']) > 0:
        action = commit['body'][0]
        if action.get('method') == 'send':
            print('${GREEN}✓ SEND commit has correct method${NC}')
            sys.exit(0)
sys.exit(1)
" && TESTS_PASSED=$((TESTS_PASSED + 1)) || TESTS_FAILED=$((TESTS_FAILED + 1))
    
    python3 -c "
import json, sys
with open('data/alice/.contract/commits/${SEND_COMMIT_ID}.json') as f:
    commit = json.load(f)
    if commit.get('body') and len(commit['body']) > 0:
        action = commit['body'][0]
        if action.get('value', {}).get('amount') == 10000:
            print('${GREEN}✓ SEND commit has correct amount${NC}')
            sys.exit(0)
sys.exit(1)
" && TESTS_PASSED=$((TESTS_PASSED + 1)) || TESTS_FAILED=$((TESTS_FAILED + 1))
else
    echo "${RED}✗ SEND commit file not found${NC}"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi

# Check Bob's RECV commit
RECV_COMMIT_ID=$(cat data/bob/recv-tokens.json | python3 -c "import sys, json; print(json.load(sys.stdin)['commit_id'])")
if [ -f "data/bob/.contract/commits/${RECV_COMMIT_ID}.json" ]; then
    echo "${GREEN}✓ RECV commit file exists${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
    
    python3 -c "
import json, sys
with open('data/bob/.contract/commits/${RECV_COMMIT_ID}.json') as f:
    commit = json.load(f)
    if commit.get('body') and len(commit['body']) > 0:
        action = commit['body'][0]
        if action.get('method') == 'recv':
            print('${GREEN}✓ RECV commit has correct method${NC}')
            sys.exit(0)
sys.exit(1)
" && TESTS_PASSED=$((TESTS_PASSED + 1)) || TESTS_FAILED=$((TESTS_FAILED + 1))
    
    python3 -c "
import json, sys
with open('data/bob/.contract/commits/${RECV_COMMIT_ID}.json') as f:
    commit = json.load(f)
    send_id = '${SEND_COMMIT_ID}'
    if commit.get('body') and len(commit['body']) > 0:
        action = commit['body'][0]
        if action.get('value', {}).get('send_commit_id') == send_id:
            print('${GREEN}✓ RECV commit references correct SEND commit${NC}')
            sys.exit(0)
sys.exit(1)
" && TESTS_PASSED=$((TESTS_PASSED + 1)) || TESTS_FAILED=$((TESTS_FAILED + 1))
else
    echo "${RED}✗ RECV commit file not found${NC}"
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

