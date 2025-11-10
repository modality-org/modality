#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

# View network storage and statistics
echo "=== Network Storage Info (Validator 1) ==="
modal net storage --config ./configs/validator1.json

