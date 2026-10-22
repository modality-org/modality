#!/usr/bin/env bash
# Regression checks for test assertions: matching error output must not pass.
set -euo pipefail
cd "$(dirname "$0")"
source ./test-lib.sh

LOG_DIR=$(mktemp -d)
trap 'test_cleanup; rm -rf "$LOG_DIR"' EXIT
CURRENT_LOG="$LOG_DIR/assertions.log"
: > "$CURRENT_LOG"

assert_output_contains "printf 'ready\\n'" ready "successful output matches"
if assert_output_contains "printf 'ready\\n'; exit 7" ready "failed command must not match"; then
    echo 'assert_output_contains accepted a failed command' >&2
    exit 1
fi
if assert_output_contains "printf 'not-ready\\n'" '^ready$' "missing pattern must not match"; then
    echo 'assert_output_contains accepted a missing pattern' >&2
    exit 1
fi
[ "$TESTS_RUN" -eq 3 ] && [ "$TESTS_PASSED" -eq 1 ] && [ "$TESTS_FAILED" -eq 2 ]
echo 'network assertion self-test passed'
