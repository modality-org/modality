#!/bin/bash
# Run formula tests for Modality language
# Tests modal operators, temporal operators, and fixed points

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PASS=0
FAIL=0

run_test() {
    local file=$1
    local model=$2
    local formula=$3
    local expected=$4  # "pass" or "fail"
    
    echo -n "Testing $model::$formula... "
    
    if modality model check "$file" --model "$model" --formula "$formula" > /dev/null 2>&1; then
        if [ "$expected" = "pass" ]; then
            echo -e "${GREEN}PASS${NC}"
            ((PASS++))
        else
            echo -e "${RED}FAIL (expected to fail)${NC}"
            ((FAIL++))
        fi
    else
        if [ "$expected" = "fail" ]; then
            echo -e "${GREEN}PASS (correctly failed)${NC}"
            ((PASS++))
        else
            echo -e "${RED}FAIL${NC}"
            ((FAIL++))
        fi
    fi
}

echo "========================================"
echo "Modality Formula Tests"
echo "========================================"
echo

echo "--- Modal Operators ---"
run_test modal-operators.modality Escrow CanDeposit pass
run_test modal-operators.modality Escrow AllDeliverOk pass
run_test modal-operators.modality Escrow CanComplete pass
run_test modal-operators.modality DiamondBoxDemo CommittedToPay pass
run_test modal-operators.modality Escrow AllSuccessorsOk pass
run_test modal-operators.modality Escrow HasSuccessor pass
echo

echo "--- Temporal Operators ---"
run_test temporal-operators.modality SimpleLoop AlwaysSafe pass
run_test temporal-operators.modality Reachability EventuallyGoal pass
run_test temporal-operators.modality Reachability NextMiddle pass
run_test temporal-operators.modality LivenessDemo CanStartWorking pass
echo

echo "--- Fixed Points (Mu-Calculus) ---"
run_test fixed-points.modality Reachable ReachTarget pass
run_test fixed-points.modality Reachable EventuallyTarget pass
run_test fixed-points.modality Reachable ReachTerminal pass
run_test fixed-points.modality Cycle AlwaysInCycle pass
echo

echo "========================================"
echo -e "Results: ${GREEN}$PASS passed${NC}, ${RED}$FAIL failed${NC}"
echo "========================================"

if [ $FAIL -gt 0 ]; then
    exit 1
fi
