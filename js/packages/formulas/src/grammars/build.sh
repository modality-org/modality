#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR=$(dirname "${BASH_SOURCE[0]}")

ANTLR_BIN=$(command -v antlr || command -v antlr4 || true)
if [[ -z "${ANTLR_BIN}" ]]; then
  echo "error: antlr or antlr4 is required to regenerate the Modality parser" >&2
  exit 127
fi

cd "${SCRIPT_DIR}"
"${ANTLR_BIN}" -Dlanguage=JavaScript ../../../../../grammars/antlr4/Modality.g4 -o ./build -Xexact-output-dir -visitor
