#!/usr/bin/env bash
# Package registry build moved to sites/get.modality.org/
set -euo pipefail
exec "$(cd "$(dirname "$0")/../get.modality.org" && pwd)/build-registry.sh" "$@"
