#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MODAL_BIN="${MODAL_BIN:-$ROOT_DIR/rust/target/debug/modal}"
MODAL_HELP_SURFACE="${MODAL_HELP_SURFACE:-lean}"

if [[ ! -x "$MODAL_BIN" ]]; then
  cat >&2 <<EOF
modal binary not found at: $MODAL_BIN

Build it first:
  cd "$ROOT_DIR/rust"
  cargo build -p modal --no-default-features --features contract-onboarding

Or pass an existing binary:
  MODAL_BIN=/path/to/modal $0
EOF
  exit 2
fi

TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

HELP="$TMP_DIR/modal-help.txt"
CONTRACT_HELP="$TMP_DIR/modal-contract-help.txt"
CONTRACT_ALIAS_HELP="$TMP_DIR/modal-c-help.txt"

"$MODAL_BIN" --help >"$HELP"
"$MODAL_BIN" contract --help >"$CONTRACT_HELP"
"$MODAL_BIN" c --help >"$CONTRACT_ALIAS_HELP"

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
    echo "expected help to omit command: $command_name" >&2
    cat "$help_file" >&2
    exit 1
  fi
}

for command_name in contract id passfile status commit set; do
  require_command "$command_name" "$HELP"
done

for command_name in create checkout commit set set-named-id status log; do
  require_command "$command_name" "$CONTRACT_HELP"
  require_command "$command_name" "$CONTRACT_ALIAS_HELP"
done

case "$MODAL_HELP_SURFACE" in
  lean)
    for command_name in hub node local net run predicate program chain killall upgrade; do
      reject_command "$command_name" "$HELP"
    done
    ;;
  full)
    for command_name in hub node predicate program chain; do
      require_command "$command_name" "$HELP"
    done
    ;;
  *)
    echo "unsupported MODAL_HELP_SURFACE: $MODAL_HELP_SURFACE" >&2
    echo "expected: lean or full" >&2
    exit 2
    ;;
esac

echo "modal $MODAL_HELP_SURFACE help surface check passed"
