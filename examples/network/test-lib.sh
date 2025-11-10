#!/usr/bin/env bash
# Test Library for Network Examples
# This library provides utilities for running examples as integration tests

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test state
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0
CURRENT_TEST=""

# Process management
PIDS=()

# Logging
LOG_DIR="${LOG_DIR:-./tmp/test-logs}"
CURRENT_LOG=""

# Initialize test environment
test_init() {
    local test_name="$1"
    CURRENT_TEST="$test_name"
    mkdir -p "$LOG_DIR"
    CURRENT_LOG="$LOG_DIR/${test_name}.log"
    echo "=== Test: $test_name ===" > "$CURRENT_LOG"
    echo -e "${BLUE}▶ Running: $test_name${NC}"
}

# Clean up processes
test_cleanup() {
    if [ ${#PIDS[@]} -gt 0 ]; then
        echo -e "${YELLOW}  Cleaning up ${#PIDS[@]} processes...${NC}"
        for pid in "${PIDS[@]}"; do
            if kill -0 "$pid" 2>/dev/null; then
                kill "$pid" 2>/dev/null || true
                # Give process time to exit gracefully
                sleep 0.5
                # Force kill if still running
                kill -9 "$pid" 2>/dev/null || true
            fi
        done
        PIDS=()
    fi
}

# Start a background process and track it
test_start_process() {
    local cmd="$1"
    local name="${2:-process}"
    local log_file="$LOG_DIR/${CURRENT_TEST}_${name}.log"
    
    echo "  Starting $name..." >> "$CURRENT_LOG"
    echo "  Command: $cmd" >> "$CURRENT_LOG"
    
    # Start process in background, redirect output to log
    eval "$cmd" > "$log_file" 2>&1 &
    local pid=$!
    PIDS+=("$pid")
    
    echo "  PID: $pid" >> "$CURRENT_LOG"
    echo "$pid"
}

# Wait for a process to start (check for port or log message)
test_wait_for_port() {
    local port="$1"
    local timeout="${2:-30}"
    local elapsed=0
    
    echo "  Waiting for port $port..." >> "$CURRENT_LOG"
    
    while [ $elapsed -lt $timeout ]; do
        if lsof -i ":$port" -sTCP:LISTEN -t >/dev/null 2>&1; then
            echo "  Port $port ready after ${elapsed}s" >> "$CURRENT_LOG"
            return 0
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done
    
    echo "  Timeout waiting for port $port" >> "$CURRENT_LOG"
    return 1
}

# Wait for log message
test_wait_for_log() {
    local log_file="$1"
    local pattern="$2"
    local timeout="${3:-30}"
    local elapsed=0
    
    echo "  Waiting for log pattern: $pattern" >> "$CURRENT_LOG"
    
    while [ $elapsed -lt $timeout ]; do
        if [ -f "$log_file" ] && grep -q "$pattern" "$log_file"; then
            echo "  Found pattern after ${elapsed}s" >> "$CURRENT_LOG"
            return 0
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done
    
    echo "  Timeout waiting for pattern: $pattern" >> "$CURRENT_LOG"
    return 1
}

# Assert command succeeds
assert_success() {
    local cmd="$1"
    local msg="${2:-Command should succeed}"
    
    TESTS_RUN=$((TESTS_RUN + 1))
    echo "  Test: $msg" >> "$CURRENT_LOG"
    echo "  Command: $cmd" >> "$CURRENT_LOG"
    
    if eval "$cmd" >> "$CURRENT_LOG" 2>&1; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $msg"
        echo "  Result: PASS" >> "$CURRENT_LOG"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "  ${RED}✗${NC} $msg"
        echo "  Result: FAIL" >> "$CURRENT_LOG"
        return 1
    fi
}

# Assert command fails
assert_failure() {
    local cmd="$1"
    local msg="${2:-Command should fail}"
    
    TESTS_RUN=$((TESTS_RUN + 1))
    echo "  Test: $msg" >> "$CURRENT_LOG"
    echo "  Command: $cmd" >> "$CURRENT_LOG"
    
    if ! eval "$cmd" >> "$CURRENT_LOG" 2>&1; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $msg"
        echo "  Result: PASS" >> "$CURRENT_LOG"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "  ${RED}✗${NC} $msg"
        echo "  Result: FAIL" >> "$CURRENT_LOG"
        return 1
    fi
}

# Assert output contains pattern
assert_output_contains() {
    local cmd="$1"
    local pattern="$2"
    local msg="${3:-Output should contain: $pattern}"
    
    TESTS_RUN=$((TESTS_RUN + 1))
    echo "  Test: $msg" >> "$CURRENT_LOG"
    echo "  Command: $cmd" >> "$CURRENT_LOG"
    echo "  Expected pattern: $pattern" >> "$CURRENT_LOG"
    
    local output
    output=$(eval "$cmd" 2>&1)
    echo "  Output: $output" >> "$CURRENT_LOG"
    
    if echo "$output" | grep -q "$pattern"; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $msg"
        echo "  Result: PASS" >> "$CURRENT_LOG"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "  ${RED}✗${NC} $msg"
        echo "  Result: FAIL (pattern not found)" >> "$CURRENT_LOG"
        return 1
    fi
}

# Assert file exists
assert_file_exists() {
    local file="$1"
    local msg="${2:-File should exist: $file}"
    
    TESTS_RUN=$((TESTS_RUN + 1))
    echo "  Test: $msg" >> "$CURRENT_LOG"
    
    if [ -f "$file" ] || [ -d "$file" ]; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $msg"
        echo "  Result: PASS" >> "$CURRENT_LOG"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "  ${RED}✗${NC} $msg"
        echo "  Result: FAIL (not found)" >> "$CURRENT_LOG"
        return 1
    fi
}

# Assert numeric comparison
assert_number() {
    local actual="$1"
    local operator="$2"
    local expected="$3"
    local msg="${4:-$actual $operator $expected}"
    
    TESTS_RUN=$((TESTS_RUN + 1))
    echo "  Test: $msg" >> "$CURRENT_LOG"
    echo "  Comparison: $actual $operator $expected" >> "$CURRENT_LOG"
    
    local result=false
    case "$operator" in
        "==" | "-eq") [ "$actual" -eq "$expected" ] && result=true ;;
        "!=" | "-ne") [ "$actual" -ne "$expected" ] && result=true ;;
        ">" | "-gt") [ "$actual" -gt "$expected" ] && result=true ;;
        "<" | "-lt") [ "$actual" -lt "$expected" ] && result=true ;;
        ">=" | "-ge") [ "$actual" -ge "$expected" ] && result=true ;;
        "<=" | "-le") [ "$actual" -le "$expected" ] && result=true ;;
        *) echo "Unknown operator: $operator"; return 1 ;;
    esac
    
    if [ "$result" = true ]; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "  ${GREEN}✓${NC} $msg"
        echo "  Result: PASS" >> "$CURRENT_LOG"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "  ${RED}✗${NC} $msg"
        echo "  Result: FAIL" >> "$CURRENT_LOG"
        return 1
    fi
}

# Finalize test
test_finalize() {
    test_cleanup
    
    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}✓ $CURRENT_TEST passed ($TESTS_PASSED/$TESTS_RUN tests)${NC}"
        return 0
    else
        echo -e "${RED}✗ $CURRENT_TEST failed ($TESTS_FAILED/$TESTS_RUN tests failed)${NC}"
        echo -e "  Log: $CURRENT_LOG"
        return 1
    fi
}

# Print test summary
test_summary() {
    echo ""
    echo "================================"
    echo "Test Summary"
    echo "================================"
    echo "Total tests: $TESTS_RUN"
    echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
    if [ $TESTS_FAILED -gt 0 ]; then
        echo -e "${RED}Failed: $TESTS_FAILED${NC}"
    else
        echo "Failed: 0"
    fi
    echo "Logs: $LOG_DIR"
    echo ""
    
    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}All tests passed!${NC}"
        return 0
    else
        echo -e "${RED}Some tests failed.${NC}"
        return 1
    fi
}

# Trap to ensure cleanup on exit
trap test_cleanup EXIT INT TERM

