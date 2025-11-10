#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

echo "=== Validator 1 Info ==="
modal node info --config ./configs/validator1.json --verbose

echo ""
echo "=== Validator 2 Info ==="
modal node info --config ./configs/validator2.json --verbose

echo ""
echo "=== Validator 3 Info ==="
modal node info --config ./configs/validator3.json --verbose

