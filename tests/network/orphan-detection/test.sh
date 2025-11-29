#!/bin/bash
set -e

# Test script for orphan detection
# Tests both the modal chain validate CLI command and the unit tests in modal-miner

TEST_NAME="orphan-detection"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=============================================="
echo "  Testing: $TEST_NAME"
echo "=============================================="
echo ""

# Test 1: Run modal-miner unit tests
echo "Test 1: Run modal-miner orphan detection unit tests"
cd "$SCRIPT_DIR/../../../rust/modal-miner"
if cargo test --features persistence --lib orphan_detection > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Unit tests passed (5 tests)"
else
    echo -e "${RED}✗${NC} Unit tests failed"
    exit 1
fi
cd "$SCRIPT_DIR"
echo ""

# Test 2: Run all CLI validation tests
echo "Test 2: Run all CLI validation tests"
if modal chain validate > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} All CLI validation tests passed"
else
    echo -e "${RED}✗${NC} CLI validation tests failed"
    exit 1
fi
echo ""

# Test 3: Run specific CLI tests
echo "Test 3: Run specific CLI tests (fork and gap)"
if modal chain validate --test fork --test gap > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Specific CLI tests passed"
else
    echo -e "${RED}✗${NC} Specific CLI tests failed"
    exit 1
fi
echo ""

# Test 4: Test JSON output
echo "Test 4: Verify JSON output format"
JSON_OUTPUT=$(modal chain validate --test fork --json)
if echo "$JSON_OUTPUT" | jq -e '.summary.passed > 0' > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} JSON output is valid and contains results"
else
    echo -e "${RED}✗${NC} JSON output is invalid"
    exit 1
fi
echo ""

# Test 5: Run each individual CLI test
echo "Test 5: Run each CLI test individually"
TESTS=("fork" "gap" "missing-parent" "integrity" "promotion")
for test in "${TESTS[@]}"; do
    if modal chain validate --test "$test" > /dev/null 2>&1; then
        echo -e "  ${GREEN}✓${NC} $test passed"
    else
        echo -e "  ${RED}✗${NC} $test failed"
        exit 1
    fi
done
echo ""

# Test 6: Verify command help output
echo "Test 6: Verify help output"
if modal chain validate --help | grep -q "Validate blockchain orphaning logic"; then
    echo -e "${GREEN}✓${NC} Help output is correct"
else
    echo -e "${RED}✗${NC} Help output is missing or incorrect"
    exit 1
fi
echo ""

# Summary
echo "=============================================="
echo -e "${GREEN}✓ All $TEST_NAME tests passed!${NC}"
echo "  - Unit tests: 5 passed"
echo "  - CLI tests: 5 passed"
echo "=============================================="

