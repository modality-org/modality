#!/usr/bin/env bash
set -euo pipefail

ARTIFACT_DIR="${1:-${MODAL_ONBOARDING_ARTIFACT_DIR:-}}"
if [[ -z "$ARTIFACT_DIR" ]]; then
  cat >&2 <<EOF
usage: $0 /path/to/downloaded-artifact-dir

Pass the directory produced by:
  gh run download <run-id> --name <artifact-name> --dir /path/to/downloaded-artifact-dir

Set MODAL_ONBOARDING_ARTIFACT_EXPECT_REV=<commit> when the downloaded artifact
must prove it came from one exact source revision.
EOF
  exit 2
fi
if [[ ! -d "$ARTIFACT_DIR" ]]; then
  echo "release artifact download directory does not exist: $ARTIFACT_DIR" >&2
  exit 2
fi

shopt -s nullglob
archives=("$ARTIFACT_DIR"/modal-*.tar.gz)
sidecars=("$ARTIFACT_DIR"/modal-*.tar.gz.sha256)
shopt -u nullglob

if [[ "${#archives[@]}" -ne 1 ]]; then
  printf 'expected exactly one modal release archive in %s, found %s\n' \
    "$ARTIFACT_DIR" "${#archives[@]}" >&2
  exit 1
fi
if [[ "${#sidecars[@]}" -ne 1 ]]; then
  printf 'expected exactly one detached archive checksum in %s, found %s\n' \
    "$ARTIFACT_DIR" "${#sidecars[@]}" >&2
  exit 1
fi

archive_path="${archives[0]}"
sidecar_path="${sidecars[0]}"
archive_name="$(basename "$archive_path")"
expected_sidecar_name="$archive_name.sha256"
if [[ "$(basename "$sidecar_path")" != "$expected_sidecar_name" ]]; then
  cat >&2 <<EOF
release artifact checksum sidecar does not match archive name
archive:  $archive_name
sidecar:  $(basename "$sidecar_path")
expected: $expected_sidecar_name
EOF
  exit 1
fi

(
  cd "$ARTIFACT_DIR"
  sha256sum -c "$expected_sidecar_name" >/dev/null
)

archive_listing="$(tar -tzf "$archive_path" | sort)"
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
release artifact archive has unexpected entries
expected:
$expected_archive_listing
actual:
$archive_listing
EOF
  exit 1
fi

unpack_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$unpack_dir"
}
trap cleanup EXIT

tar -C "$unpack_dir" -xzf "$archive_path"
checksum_entries="$(
  cd "$unpack_dir"
  awk '{ print $2 }' SHA256SUMS | sort
)"
expected_checksum_entries="$(
  printf '%s\n' "EVIDENCE-BUNDLE.txt" "PROVENANCE.txt" "README.txt" "bin/modal" | sort
)"
if [[ "$checksum_entries" != "$expected_checksum_entries" ]]; then
  cat >&2 <<EOF
release artifact checksum manifest has unexpected entries
expected:
$expected_checksum_entries
actual:
$checksum_entries
EOF
  exit 1
fi
(
  cd "$unpack_dir"
  sha256sum -c SHA256SUMS >/dev/null
)

if [[ ! -x "$unpack_dir/bin/modal" ]]; then
  echo "release artifact unpacked modal is not executable" >&2
  exit 1
fi
if ! grep -Fq "modal replayable evidence bundle" "$unpack_dir/EVIDENCE-BUNDLE.txt"; then
  echo "release artifact evidence manifest is missing its bundle marker" >&2
  exit 1
fi
if ! grep -Fq "artifact: $archive_name" "$unpack_dir/EVIDENCE-BUNDLE.txt"; then
  echo "release artifact evidence manifest is missing artifact: $archive_name" >&2
  exit 1
fi
if ! grep -Fq "source revision:" "$unpack_dir/PROVENANCE.txt"; then
  echo "release artifact provenance is missing source revision" >&2
  exit 1
fi
provenance_revision="$(
  awk -F': ' '/^source revision: / { print $2 }' "$unpack_dir/PROVENANCE.txt"
)"
if [[ -z "$provenance_revision" ]]; then
  echo "release artifact provenance has an empty source revision" >&2
  exit 1
fi
if [[ -n "${MODAL_ONBOARDING_ARTIFACT_EXPECT_REV:-}" && "$provenance_revision" != "$MODAL_ONBOARDING_ARTIFACT_EXPECT_REV" ]]; then
  cat >&2 <<EOF
release artifact source revision mismatch
expected: $MODAL_ONBOARDING_ARTIFACT_EXPECT_REV
actual:   $provenance_revision

Download the artifact from the expected workflow run or unset
MODAL_ONBOARDING_ARTIFACT_EXPECT_REV for a smoke-only artifact shape check.
EOF
  exit 1
fi
if ! grep -Fq "source revision: $provenance_revision" "$unpack_dir/EVIDENCE-BUNDLE.txt"; then
  echo "release artifact evidence manifest is missing source revision: $provenance_revision" >&2
  exit 1
fi

if [[ "${MODAL_ONBOARDING_ARTIFACT_SMOKE:-0}" == "1" ]]; then
  ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  MODAL_BIN="$unpack_dir/bin/modal" "$ROOT_DIR/tests/cli/check-modal-help-surface.sh"
  if [[ -x "${MODALITY_BIN:-}" ]]; then
    MODAL_BIN="$unpack_dir/bin/modal" MODALITY_BIN="$MODALITY_BIN" \
      "$ROOT_DIR/tests/cli/run-first-contract-cli-smoke.sh"
  fi
fi

echo "modal release artifact download check passed: $archive_name"
