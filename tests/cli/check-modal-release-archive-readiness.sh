#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PROFILE="${MODAL_ONBOARDING_PROFILE:-debug}"
HELP_SURFACE="${MODAL_HELP_SURFACE:-lean}"

if [[ -z "${MODAL_BIN:-}" ]]; then
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

  case "$PROFILE" in
    debug)
      MODAL_BIN="$CARGO_OUTPUT_DIR/debug/modal"
      ;;
    release)
      MODAL_BIN="$CARGO_OUTPUT_DIR/release/modal"
      ;;
    *)
      echo "unsupported MODAL_ONBOARDING_PROFILE: $PROFILE" >&2
      echo "expected: debug or release" >&2
      exit 2
      ;;
  esac
fi

if [[ ! -x "$MODAL_BIN" ]]; then
  cat >&2 <<EOF
release archive readiness check needs a built modal binary at $MODAL_BIN

Build it first, or run from the root smoke with:
  MODAL_ONBOARDING_BUILD=1 MODAL_ONBOARDING_ARCHIVE_CHECK=1 tests/run-onboarding-smokes.sh

Or pass an explicit binary:
  MODAL_BIN=/path/to/modal $0
EOF
  exit 2
fi

version_output="$("$MODAL_BIN" --version)"
case "$version_output" in
  modal\ [0-9]*)
    version="${version_output#modal }"
    ;;
  *)
    echo "modal reported unexpected version output: $version_output" >&2
    exit 1
    ;;
esac

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"
version_slug="$(printf '%s' "$version" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9._-]+/-/g; s/^-+//; s/-+$//')"
if [[ -z "$version_slug" ]]; then
  echo "modal version did not produce a usable archive slug: $version_output" >&2
  exit 1
fi
archive_name="modal-${version_slug}-${os}-${arch}-${PROFILE}.tar.gz"
source_revision="${MODAL_ONBOARDING_ARCHIVE_REV:-}"
version_revision_pattern='@([^)]+)\)'
if [[ -z "$source_revision" && "$version_output" =~ $version_revision_pattern ]]; then
  source_revision="${BASH_REMATCH[1]}"
fi
if [[ -z "$source_revision" ]] && git -C "$ROOT_DIR" rev-parse --verify HEAD >/dev/null 2>&1; then
  source_revision="$(git -C "$ROOT_DIR" rev-parse HEAD)"
fi
if [[ -z "$source_revision" ]]; then
  source_revision="unknown"
fi
if [[ -n "${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" && "$source_revision" != "$MODAL_ONBOARDING_ARCHIVE_EXPECT_REV" ]]; then
  cat >&2 <<EOF
release archive source revision mismatch
expected: $MODAL_ONBOARDING_ARCHIVE_EXPECT_REV
actual:   $source_revision

Rebuild modal from the expected revision, or unset
MODAL_ONBOARDING_ARCHIVE_EXPECT_REV for a smoke-only archive shape check.
EOF
  exit 1
fi

if [[ -n "${MODAL_ONBOARDING_ARCHIVE_DIR:-}" ]]; then
  ARCHIVE_DIR="$MODAL_ONBOARDING_ARCHIVE_DIR"
  mkdir -p "$ARCHIVE_DIR"
  CLEAN_ARCHIVE_DIR=0
else
  ARCHIVE_DIR="$(mktemp -d)"
  CLEAN_ARCHIVE_DIR=1
fi

STAGE_DIR="$(mktemp -d)"
UNPACK_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$STAGE_DIR" "$UNPACK_DIR"
  if [[ "$CLEAN_ARCHIVE_DIR" == "1" ]]; then
    rm -rf "$ARCHIVE_DIR"
  fi
}
trap cleanup EXIT

mkdir -p "$STAGE_DIR/bin"
cp "$MODAL_BIN" "$STAGE_DIR/bin/modal"
chmod 0755 "$STAGE_DIR/bin/modal"
cat >"$STAGE_DIR/README.txt" <<EOF
modal release archive smoke artifact
version: $version_output
profile: $PROFILE
help surface: $HELP_SURFACE
EOF
cat >"$STAGE_DIR/PROVENANCE.txt" <<EOF
modal release archive smoke provenance
source revision: $source_revision
version: $version_output
profile: $PROFILE
features: ${MODAL_ONBOARDING_FEATURES:-contract-onboarding}
help surface: $HELP_SURFACE
os: $os
arch: $arch
EOF
cat >"$STAGE_DIR/EVIDENCE-BUNDLE.txt" <<EOF
modal replayable evidence bundle
artifact: $archive_name
source revision: $source_revision
binary: bin/modal
provenance: PROVENANCE.txt
checksums: SHA256SUMS
post-unpack checks: version, help surface, first-contract smoke when MODALITY_BIN is supplied
EOF
(
  cd "$STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
)

ARCHIVE_PATH="$ARCHIVE_DIR/$archive_name"
tar -C "$STAGE_DIR" -czf "$ARCHIVE_PATH" \
  bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS

archive_listing="$(tar -tzf "$ARCHIVE_PATH" | sort)"
expected_archive_listing="$(
  printf '%s\n' \
    "EVIDENCE-BUNDLE.txt" \
    "PROVENANCE.txt" \
    "README.txt" \
    "SHA256SUMS" \
    "bin/modal" | sort
)"
if [[ "$archive_listing" != "$expected_archive_listing" ]]; then
  cat >&2 <<EOF
release archive has unexpected entries
expected:
$expected_archive_listing
actual:
$archive_listing
EOF
  exit 1
fi
if ! grep -Fxq "README.txt" <<<"$archive_listing"; then
  echo "release archive is missing README.txt" >&2
  exit 1
fi
if ! grep -Fxq "bin/modal" <<<"$archive_listing"; then
  echo "release archive is missing bin/modal" >&2
  exit 1
fi
if ! grep -Fxq "PROVENANCE.txt" <<<"$archive_listing"; then
  echo "release archive is missing PROVENANCE.txt" >&2
  exit 1
fi
if ! grep -Fxq "EVIDENCE-BUNDLE.txt" <<<"$archive_listing"; then
  echo "release archive is missing EVIDENCE-BUNDLE.txt" >&2
  exit 1
fi
if ! grep -Fxq "SHA256SUMS" <<<"$archive_listing"; then
  echo "release archive is missing SHA256SUMS" >&2
  exit 1
fi

tar -C "$UNPACK_DIR" -xzf "$ARCHIVE_PATH"
checksum_entries="$(
  cd "$UNPACK_DIR"
  awk '{ print $2 }' SHA256SUMS | sort
)"
expected_checksum_entries="$(
  printf '%s\n' "EVIDENCE-BUNDLE.txt" "PROVENANCE.txt" "README.txt" "bin/modal" | sort
)"
if [[ "$checksum_entries" != "$expected_checksum_entries" ]]; then
  cat >&2 <<EOF
release archive checksum manifest has unexpected entries
expected:
$expected_checksum_entries
actual:
$checksum_entries
EOF
  exit 1
fi
(
  cd "$UNPACK_DIR"
  sha256sum -c SHA256SUMS >/dev/null
)
if ! grep -Fq "source revision: $source_revision" "$UNPACK_DIR/PROVENANCE.txt"; then
  echo "release archive provenance is missing source revision: $source_revision" >&2
  exit 1
fi
if ! grep -Fq "profile: $PROFILE" "$UNPACK_DIR/PROVENANCE.txt"; then
  echo "release archive provenance is missing profile: $PROFILE" >&2
  exit 1
fi
if ! grep -Fq "help surface: $HELP_SURFACE" "$UNPACK_DIR/PROVENANCE.txt"; then
  echo "release archive provenance is missing help surface: $HELP_SURFACE" >&2
  exit 1
fi
if ! grep -Fq "modal replayable evidence bundle" "$UNPACK_DIR/EVIDENCE-BUNDLE.txt"; then
  echo "release archive evidence manifest is missing its bundle marker" >&2
  exit 1
fi
if ! grep -Fq "artifact: $archive_name" "$UNPACK_DIR/EVIDENCE-BUNDLE.txt"; then
  echo "release archive evidence manifest is missing artifact: $archive_name" >&2
  exit 1
fi
if ! grep -Fq "source revision: $source_revision" "$UNPACK_DIR/EVIDENCE-BUNDLE.txt"; then
  echo "release archive evidence manifest is missing source revision: $source_revision" >&2
  exit 1
fi
if ! grep -Fq "post-unpack checks: version, help surface, first-contract smoke when MODALITY_BIN is supplied" "$UNPACK_DIR/EVIDENCE-BUNDLE.txt"; then
  echo "release archive evidence manifest is missing post-unpack checks" >&2
  exit 1
fi
UNPACKED_MODAL="$UNPACK_DIR/bin/modal"
if [[ ! -x "$UNPACKED_MODAL" ]]; then
  echo "unpacked modal is not executable at $UNPACKED_MODAL" >&2
  exit 1
fi

unpacked_version="$("$UNPACKED_MODAL" --version)"
if [[ "$unpacked_version" != "$version_output" ]]; then
  echo "unpacked modal version changed: $unpacked_version (expected $version_output)" >&2
  exit 1
fi

MODAL_BIN="$UNPACKED_MODAL" MODAL_HELP_SURFACE="$HELP_SURFACE" \
  "$ROOT_DIR/tests/cli/check-modal-help-surface.sh"

if [[ -x "${MODALITY_BIN:-}" ]]; then
  MODAL_BIN="$UNPACKED_MODAL" MODALITY_BIN="$MODALITY_BIN" \
    "$ROOT_DIR/tests/cli/run-first-contract-cli-smoke.sh"
else
  cat <<EOF
first-contract release-archive smoke skipped: MODALITY_BIN not supplied

Pass a built language CLI to verify the unpacked modal binary against the full
first-contract path:
  MODALITY_BIN=/path/to/modality $0
EOF
fi

echo "modal release archive readiness check passed: $archive_name"
