#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

# Run validator 1
modal node run-validator --config ./configs/validator1.json

