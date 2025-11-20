#!/bin/bash
set -e

# Test script for orphan detection using modal chain validate command

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

# Test 1: Run all tests with default settings
echo "Test 1: Run all validation tests"
if modal chain validate; then
    echo -e "${GREEN}✓${NC} All validation tests passed"
else
    echo -e "${RED}✗${NC} Validation tests failed"
    exit 1
fi
echo ""

# Test 2: Run specific tests
echo "Test 2: Run specific tests (fork and gap)"
if modal chain validate --test fork --test gap; then
    echo -e "${GREEN}✓${NC} Specific tests passed"
else
    echo -e "${RED}✗${NC} Specific tests failed"
    exit 1
fi
echo ""

# Test 3: Test JSON output
echo "Test 3: Verify JSON output format"
JSON_OUTPUT=$(modal chain validate --test fork --json)
if echo "$JSON_OUTPUT" | jq -e '.summary.passed > 0' > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} JSON output is valid and contains results"
else
    echo -e "${RED}✗${NC} JSON output is invalid"
    exit 1
fi
echo ""

# Test 4: Run each individual test
echo "Test 4: Run each test individually"
TESTS=("fork" "gap" "missing-parent" "integrity" "promotion")
for test in "${TESTS[@]}"; do
    echo "  Testing: $test"
    if modal chain validate --test "$test" > /dev/null 2>&1; then
        echo -e "    ${GREEN}✓${NC} $test passed"
    else
        echo -e "    ${RED}✗${NC} $test failed"
        exit 1
    fi
done
echo ""

# Test 5: Verify command help output
echo "Test 5: Verify help output"
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
echo "=============================================="

