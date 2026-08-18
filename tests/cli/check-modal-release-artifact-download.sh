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
recipe_path="$ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
if [[ ! -f "$recipe_path" ]]; then
  printf 'expected VERIFY-DOWNLOAD.txt recipe in %s\n' "$ARTIFACT_DIR" >&2
  exit 1
fi

archive_path="${archives[0]}"
sidecar_path="${sidecars[0]}"
archive_name="$(basename "$archive_path")"
expected_sidecar_name="$archive_name.sha256"
top_level_entries="$(
  find "$ARTIFACT_DIR" -mindepth 1 -maxdepth 1 -printf '%f\n' | sort
)"
expected_top_level_entries="$(
  printf '%s\n' "$archive_name" "$expected_sidecar_name" "VERIFY-DOWNLOAD.txt" | sort
)"
if [[ "$top_level_entries" != "$expected_top_level_entries" ]]; then
  cat >&2 <<EOF
release artifact download directory has unexpected top-level entries
expected:
$expected_top_level_entries
actual:
$top_level_entries
EOF
  exit 1
fi
if [[ "$(basename "$sidecar_path")" != "$expected_sidecar_name" ]]; then
  cat >&2 <<EOF
release artifact checksum sidecar does not match archive name
archive:  $archive_name
sidecar:  $(basename "$sidecar_path")
expected: $expected_sidecar_name
EOF
  exit 1
fi
sidecar_entries="$(
  awk '{ print $2 }' "$sidecar_path" | sort
)"
if [[ "$sidecar_entries" != "$archive_name" ]]; then
  cat >&2 <<EOF
release artifact detached checksum sidecar has unexpected entries
expected:
$archive_name
actual:
$sidecar_entries
EOF
  exit 1
fi
for required_path in "$archive_path" "$sidecar_path" "$recipe_path"; do
  if [[ ! -f "$required_path" || -L "$required_path" ]]; then
    printf 'release artifact top-level entry must be a regular non-symlink file: %s\n' \
      "$(basename "$required_path")" >&2
    exit 1
  fi
done

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
for required_unpacked_path in \
  "$unpack_dir/bin/modal" \
  "$unpack_dir/README.txt" \
  "$unpack_dir/PROVENANCE.txt" \
  "$unpack_dir/EVIDENCE-BUNDLE.txt" \
  "$unpack_dir/SHA256SUMS"
do
  if [[ ! -f "$required_unpacked_path" || -L "$required_unpacked_path" ]]; then
    relative_path="${required_unpacked_path#"$unpack_dir"/}"
    printf 'release artifact unpacked entry must be a regular non-symlink file: %s\n' \
      "$relative_path" >&2
    exit 1
  fi
done
check_unpacked_mode() {
  local relative_path="$1"
  local expected_mode="$2"
  local actual_mode
  actual_mode="$(stat -c '%a' "$unpack_dir/$relative_path")"
  if [[ "$actual_mode" != "$expected_mode" ]]; then
    cat >&2 <<EOF
release artifact unpacked entry has unexpected mode: $relative_path
expected: $expected_mode
actual:   $actual_mode
EOF
    exit 1
  fi
}
check_unpacked_mode "bin/modal" "755"
check_unpacked_mode "README.txt" "644"
check_unpacked_mode "PROVENANCE.txt" "644"
check_unpacked_mode "EVIDENCE-BUNDLE.txt" "644"
check_unpacked_mode "SHA256SUMS" "644"
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
provenance_revision_count="$(
  grep -Ec '^source revision: .+' "$unpack_dir/PROVENANCE.txt" || true
)"
if [[ "$provenance_revision_count" -ne 1 ]]; then
  cat >&2 <<EOF
release artifact provenance must name exactly one source revision
actual count: $provenance_revision_count
EOF
  exit 1
fi
provenance_revision="$(
  awk -F': ' '/^source revision: / { print $2 }' "$unpack_dir/PROVENANCE.txt"
)"
if [[ -z "$provenance_revision" ]]; then
  echo "release artifact provenance has an empty source revision" >&2
  exit 1
fi
require_provenance_field() {
  local label="$1"
  local count
  local value
  count="$(
    grep -Ec "^${label}: .+" "$unpack_dir/PROVENANCE.txt" || true
  )"
  if [[ "$count" -ne 1 ]]; then
    cat >&2 <<EOF
release artifact provenance must name exactly one $label
actual count: $count
EOF
    exit 1
  fi
  value="$(
    awk -v label="$label" '
      index($0, label ": ") == 1 {
        sub(label ": ", "")
        print
      }
    ' "$unpack_dir/PROVENANCE.txt"
  )"
  if [[ -z "$value" ]]; then
    echo "release artifact provenance has an empty $label" >&2
    exit 1
  fi
}
require_provenance_field "version"
require_provenance_field "profile"
require_provenance_field "features"
require_provenance_field "help surface"
require_provenance_field "os"
require_provenance_field "arch"
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
if ! grep -Fq "binary: bin/modal" "$unpack_dir/EVIDENCE-BUNDLE.txt"; then
  echo "release artifact evidence manifest is missing binary: bin/modal" >&2
  exit 1
fi
if ! grep -Fq "provenance: PROVENANCE.txt" "$unpack_dir/EVIDENCE-BUNDLE.txt"; then
  echo "release artifact evidence manifest is missing provenance: PROVENANCE.txt" >&2
  exit 1
fi
if ! grep -Fq "checksums: SHA256SUMS" "$unpack_dir/EVIDENCE-BUNDLE.txt"; then
  echo "release artifact evidence manifest is missing checksums: SHA256SUMS" >&2
  exit 1
fi
if ! grep -Fq "post-unpack checks: version, help surface, first-contract smoke when MODALITY_BIN is supplied" "$unpack_dir/EVIDENCE-BUNDLE.txt"; then
  echo "release artifact evidence manifest is missing post-unpack checks" >&2
  exit 1
fi
if ! grep -Fq "modal release archive download verification" "$recipe_path"; then
  echo "release artifact verification recipe is missing its title" >&2
  exit 1
fi
if ! grep -Fq "Artifact:" "$recipe_path"; then
  echo "release artifact verification recipe is missing artifact label" >&2
  exit 1
fi
recipe_artifacts="$(
  awk '
    /^Artifact:$/ { in_section = 1; next }
    in_section && /^$/ { in_section = 0; next }
    in_section && /^  / { sub(/^  /, ""); print }
  ' "$recipe_path"
)"
if [[ "$recipe_artifacts" != "$archive_name" ]]; then
  cat >&2 <<EOF
release artifact verification recipe has unexpected artifacts
expected:
$archive_name
actual:
$recipe_artifacts
EOF
  exit 1
fi
if ! grep -Fq "$expected_sidecar_name" "$recipe_path"; then
  echo "release artifact verification recipe is missing checksum sidecar: $expected_sidecar_name" >&2
  exit 1
fi
if ! grep -Fq "Expected downloaded directory entries:" "$recipe_path"; then
  echo "release artifact verification recipe is missing expected directory entries label" >&2
  exit 1
fi
recipe_expected_top_level_entries="$(
  awk '
    /^Expected downloaded directory entries:$/ { in_section = 1; next }
    in_section && /^$/ { in_section = 0; next }
    in_section && /^  / { sub(/^  /, ""); print }
  ' "$recipe_path" | sort
)"
if [[ "$recipe_expected_top_level_entries" != "$expected_top_level_entries" ]]; then
  cat >&2 <<EOF
release artifact verification recipe has unexpected downloaded directory entries
expected:
$expected_top_level_entries
actual:
$recipe_expected_top_level_entries
EOF
  exit 1
fi
if ! grep -Fq "Expected source revision:" "$recipe_path"; then
  echo "release artifact verification recipe is missing expected source revision label" >&2
  exit 1
fi
recipe_expected_revisions="$(
  awk '
    /^Expected source revision:$/ { in_section = 1; next }
    in_section && /^$/ { in_section = 0; next }
    in_section && /^  / { sub(/^  /, ""); print }
  ' "$recipe_path"
)"
if [[ "$recipe_expected_revisions" != "$provenance_revision" ]]; then
  cat >&2 <<EOF
release artifact verification recipe has unexpected source revisions
expected:
$provenance_revision
actual:
$recipe_expected_revisions
EOF
  exit 1
fi
if ! grep -Fq "sha256sum -c $expected_sidecar_name" "$recipe_path"; then
  echo "release artifact verification recipe is missing detached checksum command" >&2
  exit 1
fi
expected_verify_command="MODAL_ONBOARDING_ARTIFACT_EXPECT_REV=$provenance_revision tests/cli/check-modal-release-artifact-download.sh /path/to/downloaded-artifact-dir"
if ! grep -Fq "$expected_verify_command" "$recipe_path"; then
  echo "release artifact verification recipe is missing exact verifier command" >&2
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
