#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR=$(dirname "${BASH_SOURCE[0]}")

ANTLR_BIN=$(command -v antlr || command -v antlr4 || command -v antlr-ng || true)
if [[ -z "${ANTLR_BIN}" ]]; then
  echo "error: antlr, antlr4, or antlr-ng is required to regenerate the Modality parser" >&2
  exit 127
fi

cd "${SCRIPT_DIR}"
if [[ "$(basename "${ANTLR_BIN}")" == "antlr-ng" ]]; then
  "${ANTLR_BIN}" -Dlanguage=JavaScript ../../../../../grammars/antlr4/Modality.g4 -o ./build --exact-output-dir --generate-visitor true
else
  "${ANTLR_BIN}" -Dlanguage=JavaScript ../../../../../grammars/antlr4/Modality.g4 -o ./build -Xexact-output-dir -visitor
fi
