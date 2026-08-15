#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MIN_FREE_KB="${MODAL_ONBOARDING_MIN_KB:-1048576}"
BUILD_MODALITY="${MODALITY_ONBOARDING_BUILD:-0}"
BUILD_MODAL="${MODAL_ONBOARDING_BUILD:-0}"
INSTALL_MODAL="${MODAL_ONBOARDING_INSTALL:-0}"
PACKAGE_CHECK_MODAL="${MODAL_ONBOARDING_PACKAGE_CHECK:-0}"
GIT_INSTALL_CHECK_MODAL="${MODAL_ONBOARDING_GIT_INSTALL_CHECK:-0}"
ARCHIVE_CHECK_MODAL="${MODAL_ONBOARDING_ARCHIVE_CHECK:-0}"
MODAL_ONBOARDING_FEATURES="${MODAL_ONBOARDING_FEATURES:-contract-onboarding}"
MODAL_ONBOARDING_PROFILE="${MODAL_ONBOARDING_PROFILE:-debug}"

if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  case "$CARGO_TARGET_DIR" in
    /*)
      CARGO_OUTPUT_DIR="$CARGO_TARGET_DIR"
      ;;
    *)
      CARGO_OUTPUT_DIR="$ROOT_DIR/rust/$CARGO_TARGET_DIR"
      ;;
  esac
else
  CARGO_OUTPUT_DIR="$ROOT_DIR/rust/target"
fi

if [[ "$MODAL_ONBOARDING_FEATURES" == "full" ]]; then
  DEFAULT_MODAL_HELP_SURFACE="full"
else
  DEFAULT_MODAL_HELP_SURFACE="lean"
fi
MODAL_HELP_SURFACE="${MODAL_HELP_SURFACE:-$DEFAULT_MODAL_HELP_SURFACE}"

case "$MODAL_ONBOARDING_PROFILE" in
  debug)
    CARGO_PROFILE_ARGS=()
    CARGO_INSTALL_PROFILE_ARGS=(--debug)
    DEFAULT_MODAL_BIN="$CARGO_OUTPUT_DIR/debug/modal"
    DEFAULT_MODALITY_BIN="$CARGO_OUTPUT_DIR/debug/modality"
    ;;
  release)
    CARGO_PROFILE_ARGS=(--release)
    CARGO_INSTALL_PROFILE_ARGS=()
    DEFAULT_MODAL_BIN="$CARGO_OUTPUT_DIR/release/modal"
    DEFAULT_MODALITY_BIN="$CARGO_OUTPUT_DIR/release/modality"
    ;;
  *)
    echo "unsupported MODAL_ONBOARDING_PROFILE: $MODAL_ONBOARDING_PROFILE" >&2
    echo "expected: debug or release" >&2
    exit 2
    ;;
esac

MODAL_BIN="${MODAL_BIN:-$DEFAULT_MODAL_BIN}"
MODALITY_BIN="${MODALITY_BIN:-$DEFAULT_MODALITY_BIN}"

available_kb="$(df -Pk "$ROOT_DIR" | awk 'NR == 2 { print $4 }')"
if [[ "$available_kb" -lt "$MIN_FREE_KB" ]]; then
  cat >&2 <<EOF
onboarding smokes need at least ${MIN_FREE_KB} KiB free in $ROOT_DIR
available: ${available_kb} KiB

Free disk space, or lower the preflight only for a known no-build run:
  MODAL_ONBOARDING_MIN_KB=$available_kb $0
EOF
  exit 1
fi

if [[ ! -x "$MODALITY_BIN" && "$BUILD_MODALITY" == "1" ]]; then
  (
    cd "$ROOT_DIR/rust"
    cargo build "${CARGO_PROFILE_ARGS[@]}" -p modality
  )
fi

if [[ -x "$MODALITY_BIN" ]]; then
  export MODALITY_BIN
fi

"$ROOT_DIR/tests/language/run-onboarding-tests.sh"
"$ROOT_DIR/tests/cli/check-contract-cli-deps.sh"
"$ROOT_DIR/tests/docs/check-predicate-evidence-doc.sh"
"$ROOT_DIR/tests/docs/check-contract-evolution-doc.sh"
"$ROOT_DIR/tests/docs/check-language-traps-doc.sh"
"$ROOT_DIR/tests/docs/check-installation-doc.sh"
"$ROOT_DIR/tests/docs/check-first-contract-doc.sh"
"$ROOT_DIR/tests/docs/check-verifier-rejections-doc.sh"
"$ROOT_DIR/tests/docs/check-synthesis-review-doc.sh"
"$ROOT_DIR/tests/docs/check-acme-review-crosswalk-doc.sh"
"$ROOT_DIR/tests/language/check-modality-help-surface.sh"
"$ROOT_DIR/tests/language/check-first-contract-synthesize-cli.sh"
"$ROOT_DIR/tests/language/check-rule-synthesize-cli.sh"
"$ROOT_DIR/tests/language/check-acme-review-benchmark.sh"
"$ROOT_DIR/tests/language/check-model-lint-cli.sh"

if [[ "$PACKAGE_CHECK_MODAL" == "1" ]]; then
  "$ROOT_DIR/tests/cli/check-modal-package-readiness.sh"
fi

if [[ "$GIT_INSTALL_CHECK_MODAL" == "1" ]]; then
  "$ROOT_DIR/tests/cli/check-modal-git-install-readiness.sh"
fi

if [[ "$INSTALL_MODAL" == "1" ]]; then
  INSTALL_ROOT="$(mktemp -d)"
  cleanup_install() {
    rm -rf "$INSTALL_ROOT"
  }
  trap cleanup_install EXIT

  (
    cd "$ROOT_DIR/rust"
    cargo install \
      --path modal \
      --no-default-features \
      --features "$MODAL_ONBOARDING_FEATURES" \
      --root "$INSTALL_ROOT" \
      --locked \
      "${CARGO_INSTALL_PROFILE_ARGS[@]}"
  )
  MODAL_BIN="$INSTALL_ROOT/bin/modal"
  if [[ ! -x "$MODAL_BIN" ]]; then
    echo "cargo install did not produce executable modal at $MODAL_BIN" >&2
    exit 1
  fi
  installed_version="$("$MODAL_BIN" --version)"
  case "$installed_version" in
    modal\ [0-9]*)
      echo "installed modal smoke binary: $MODAL_BIN ($installed_version)"
      ;;
    *)
      echo "installed modal reported unexpected version output: $installed_version" >&2
      exit 1
      ;;
  esac
fi

if [[ ! -x "$MODAL_BIN" && "$BUILD_MODAL" == "1" ]]; then
  (
    cd "$ROOT_DIR/rust"
    cargo build "${CARGO_PROFILE_ARGS[@]}" -p modal --no-default-features --features "$MODAL_ONBOARDING_FEATURES"
  )
fi

if [[ "$ARCHIVE_CHECK_MODAL" == "1" ]]; then
  MODAL_BIN="$MODAL_BIN" MODAL_HELP_SURFACE="$MODAL_HELP_SURFACE" \
    "$ROOT_DIR/tests/cli/check-modal-release-archive-readiness.sh"
fi

if [[ -x "$MODAL_BIN" ]]; then
  MODAL_BIN="$MODAL_BIN" MODAL_HELP_SURFACE="$MODAL_HELP_SURFACE" \
    "$ROOT_DIR/tests/cli/check-modal-help-surface.sh"
  MODAL_BIN="$MODAL_BIN" "$ROOT_DIR/tests/cli/run-first-contract-cli-smoke.sh"
  MODAL_BIN="$MODAL_BIN" "$ROOT_DIR/tests/cli/run-contract-evolution-cli-smoke.sh"
else
  cat <<EOF
first-contract CLI smoke skipped: modal binary not found at $MODAL_BIN

Build it during the smoke:
  MODAL_ONBOARDING_BUILD=1 $0

Build both CLIs during the smoke, so language synthesis/lint checks cannot skip:
  MODALITY_ONBOARDING_BUILD=1 MODAL_ONBOARDING_BUILD=1 $0

Build and smoke the release-profile onboarding wrapper:
  MODALITY_ONBOARDING_BUILD=1 MODAL_ONBOARDING_BUILD=1 MODAL_ONBOARDING_PROFILE=release $0

Install and smoke the lean onboarding wrapper in a temporary Cargo root:
  MODAL_ONBOARDING_INSTALL=1 $0

Install and smoke the release-profile onboarding wrapper:
  MODAL_ONBOARDING_INSTALL=1 MODAL_ONBOARDING_PROFILE=release $0

Check whether the lean wrapper is ready for crates.io-style packaging:
  MODAL_ONBOARDING_PACKAGE_CHECK=1 $0

Install the lean wrapper from a Git URL into a temporary Cargo root:
  MODAL_ONBOARDING_GIT_INSTALL_CHECK=1 $0

Create and smoke a release-archive-shaped tarball from a built wrapper:
  MODAL_ONBOARDING_BUILD=1 MODAL_ONBOARDING_ARCHIVE_CHECK=1 $0

Or build it first:
  cd "$ROOT_DIR/rust"
  cargo build ${CARGO_PROFILE_ARGS[*]} -p modal --no-default-features --features "$MODAL_ONBOARDING_FEATURES"

Or pass an existing binary:
  MODAL_BIN=/path/to/modal MODALITY_BIN=/path/to/modality $0
EOF
fi
