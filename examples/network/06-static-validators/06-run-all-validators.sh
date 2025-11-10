#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

# Run all validators in the background
echo "Starting validator 1..."
./01-run-validator1.sh > tmp/validator1.log 2>&1 &
VALIDATOR1_PID=$!

echo "Starting validator 2..."
./02-run-validator2.sh > tmp/validator2.log 2>&1 &
VALIDATOR2_PID=$!

echo "Starting validator 3..."
./03-run-validator3.sh > tmp/validator3.log 2>&1 &
VALIDATOR3_PID=$!

echo ""
echo "All validators started!"
echo "Validator 1 PID: $VALIDATOR1_PID"
echo "Validator 2 PID: $VALIDATOR2_PID"
echo "Validator 3 PID: $VALIDATOR3_PID"
echo ""
echo "Logs are available at:"
echo "  - tmp/validator1.log"
echo "  - tmp/validator2.log"
echo "  - tmp/validator3.log"
echo ""
echo "To stop all validators, run: kill $VALIDATOR1_PID $VALIDATOR2_PID $VALIDATOR3_PID"

