#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

# Run validator 2
modal node run-validator --config ./configs/validator2.json

