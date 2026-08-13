#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MODALITY_BIN="${MODALITY_BIN:-$ROOT_DIR/rust/target/debug/modality}"

if [[ ! -x "$MODALITY_BIN" ]]; then
  cat <<EOF
modality help surface check skipped: modality binary not found at $MODALITY_BIN

Build it first:
  cd "$ROOT_DIR/rust"
  cargo build -p modality

Or pass an existing binary:
  MODALITY_BIN=/path/to/modality $0
EOF
  exit 0
fi

TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

VERSION="$TMP_DIR/modality-version.txt"
HELP="$TMP_DIR/modality-help.txt"
MODEL_HELP="$TMP_DIR/modality-model-help.txt"
SYNTHESIZE_HELP="$TMP_DIR/modality-model-synthesize-help.txt"
VALIDATE_HELP="$TMP_DIR/modality-model-validate-help.txt"
LINT_HELP="$TMP_DIR/modality-model-lint-help.txt"

"$MODALITY_BIN" --version >"$VERSION"
"$MODALITY_BIN" --help >"$HELP"
"$MODALITY_BIN" model --help >"$MODEL_HELP"
"$MODALITY_BIN" model synthesize --help >"$SYNTHESIZE_HELP"
"$MODALITY_BIN" model validate --help >"$VALIDATE_HELP"
"$MODALITY_BIN" model lint --help >"$LINT_HELP"

require_command() {
  local command_name="$1"
  local help_file="$2"
  if ! grep -Eq "^[[:space:]]*$command_name[[:space:]]" "$help_file"; then
    echo "expected help to include command: $command_name" >&2
    cat "$help_file" >&2
    exit 1
  fi
}

reject_command() {
  local command_name="$1"
  local help_file="$2"
  if grep -Eq "^[[:space:]]*$command_name[[:space:]]" "$help_file"; then
    echo "expected language CLI help to omit contract command: $command_name" >&2
    cat "$help_file" >&2
    exit 1
  fi
}

require_help_pattern() {
  local pattern="$1"
  local help_file="$2"
  if ! grep -Eq "$pattern" "$help_file"; then
    echo "expected help to match pattern: $pattern" >&2
    cat "$help_file" >&2
    exit 1
  fi
}

require_help_pattern "^modality[[:space:]]+[0-9]" "$VERSION"
require_command model "$HELP"

for command_name in mermaid check create synthesize validate lint; do
  require_command "$command_name" "$MODEL_HELP"
done

for command_name in contract c id passfile status commit set hub node predicate program chain; do
  reject_command "$command_name" "$HELP"
done

for required_flag in --rule --source-file --review-bundle --verify --output; do
  require_help_pattern "(^|[[:space:]])$required_flag([[:space:],=<]|$)" "$SYNTHESIZE_HELP"
done

require_help_pattern "Validate a contract model" "$VALIDATE_HELP"
require_help_pattern "Lint governance formulas" "$LINT_HELP"

echo "modality help surface check passed"
