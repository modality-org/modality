#!/usr/bin/env bash
cd $(dirname -- "$0")
set -x

# Clean up storage directories
rm -rf ./tmp/storage/validator1
rm -rf ./tmp/storage/validator2
rm -rf ./tmp/storage/validator3

echo "Storage cleaned successfully"

