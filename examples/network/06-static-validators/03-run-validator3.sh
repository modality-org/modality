#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

# Run validator 3
modal node run-validator --config ./configs/validator3.json

