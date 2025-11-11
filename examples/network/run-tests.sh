#!/usr/bin/env bash
# Main test runner for all network examples
# Runs each example's test suite in sequence

set -e
cd "$(dirname "$0")"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test results
TOTAL_SUITES=0
PASSED_SUITES=0
FAILED_SUITES=0
SKIPPED_SUITES=0

# Configuration
RUN_QUICK=${RUN_QUICK:-false}
RUN_SLOW=${RUN_SLOW:-true}
STOP_ON_FAILURE=${STOP_ON_FAILURE:-false}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --quick)
            RUN_QUICK=true
            RUN_SLOW=false
            shift
            ;;
        --all)
            RUN_QUICK=true
            RUN_SLOW=true
            shift
            ;;
        --stop-on-failure)
            STOP_ON_FAILURE=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --quick              Run only quick tests (ping, sync)"
            echo "  --all                Run all tests including slow ones (mining)"
            echo "  --stop-on-failure    Stop testing after first failure"
            echo "  --help               Show this help message"
            echo ""
            echo "Environment variables:"
            echo "  LOG_DIR             Directory for test logs (default: ./tmp/test-logs)"
            echo ""
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Run with --help for usage information"
            exit 1
            ;;
    esac
done

# Print test configuration
echo "================================"
echo "Network Examples Integration Tests"
echo "================================"
echo ""
echo "Configuration:"
echo "  Quick tests: $RUN_QUICK"
echo "  Slow tests: $RUN_SLOW"
echo "  Stop on failure: $STOP_ON_FAILURE"
echo "  Log directory: ${LOG_DIR:-./tmp/test-logs}"
echo ""

# Build modal CLI first
echo "Building modal CLI..."
if (cd ../../rust && cargo build --package modal); then
    echo -e "${GREEN}✓ Modal CLI built successfully${NC}"
else
    echo -e "${RED}✗ Failed to build modal CLI${NC}"
    exit 1
fi
echo ""

# Run test suite
run_test_suite() {
    local dir="$1"
    local name="$2"
    local category="${3:-normal}"
    
    TOTAL_SUITES=$((TOTAL_SUITES + 1))
    
    # Check if we should skip based on category
    if [ "$category" = "quick" ] && [ "$RUN_QUICK" = "false" ]; then
        echo -e "${YELLOW}⊘ Skipping $name (quick tests disabled)${NC}"
        SKIPPED_SUITES=$((SKIPPED_SUITES + 1))
        return 0
    fi
    
    if [ "$category" = "slow" ] && [ "$RUN_SLOW" = "false" ]; then
        echo -e "${YELLOW}⊘ Skipping $name (slow tests disabled)${NC}"
        SKIPPED_SUITES=$((SKIPPED_SUITES + 1))
        return 0
    fi
    
    # Check if test exists
    if [ ! -f "$dir/test.sh" ]; then
        echo -e "${YELLOW}⊘ Skipping $name (no test.sh found)${NC}"
        SKIPPED_SUITES=$((SKIPPED_SUITES + 1))
        return 0
    fi
    
    echo "================================"
    echo "Running: $name"
    echo "================================"
    
    # Run test
    if (cd "$dir" && ./test.sh); then
        PASSED_SUITES=$((PASSED_SUITES + 1))
        echo ""
        return 0
    else
        FAILED_SUITES=$((FAILED_SUITES + 1))
        echo ""
        
        if [ "$STOP_ON_FAILURE" = "true" ]; then
            echo -e "${RED}Stopping due to test failure${NC}"
            print_summary
            exit 1
        fi
        return 1
    fi
}

# Print final summary
print_summary() {
    echo ""
    echo "================================"
    echo "Test Summary"
    echo "================================"
    echo "Total suites: $TOTAL_SUITES"
    echo -e "${GREEN}Passed: $PASSED_SUITES${NC}"
    if [ $FAILED_SUITES -gt 0 ]; then
        echo -e "${RED}Failed: $FAILED_SUITES${NC}"
    else
        echo "Failed: 0"
    fi
    if [ $SKIPPED_SUITES -gt 0 ]; then
        echo -e "${YELLOW}Skipped: $SKIPPED_SUITES${NC}"
    fi
    echo ""
    
    if [ $FAILED_SUITES -eq 0 ]; then
        echo -e "${GREEN}All tests passed!${NC}"
        return 0
    else
        echo -e "${RED}Some tests failed.${NC}"
        return 1
    fi
}

# Quick tests (fast, good for CI)
run_test_suite "01-ping-node" "Ping Node" "quick"
run_test_suite "02-run-devnet1" "Run Devnet1" "quick"
run_test_suite "02-run-devnet2" "Run Devnet2" "quick"
run_test_suite "04-sync-miner-blocks" "Sync Miner Blocks" "quick"

# Normal tests (moderate duration)
run_test_suite "03-run-devnet3" "Run Devnet3" "normal"

# Slow tests (long running, might timeout in CI)
run_test_suite "05-mining" "Mining" "slow"
# run_test_suite "06-static-validators" "Static Validators" "slow"

# Print summary and exit with appropriate code
print_summary
exit $?

