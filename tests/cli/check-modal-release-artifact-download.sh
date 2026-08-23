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
check_sha256_line_shape() {
  local manifest_path="$1"
  local label="$2"
  local expected_count="$3"
  local actual_count
  local shaped_count
  actual_count="$(sed '/^$/d' "$manifest_path" | wc -l)"
  shaped_count="$(
    grep -Ec '^[0-9a-f]{64}  [^[:space:]]+$' "$manifest_path" || true
  )"
  if [[ "$actual_count" -ne "$expected_count" || "$shaped_count" -ne "$expected_count" ]]; then
    cat >&2 <<EOF
release artifact $label has unexpected checksum lines
expected count: $expected_count
actual count:   $actual_count
EOF
    exit 1
  fi
}
top_level_entries="$(
  find "$ARTIFACT_DIR" -mindepth 1 -maxdepth 1 -printf '%f\n' | sort
)"
expected_top_level_entries="$(
  printf '%s\n' "$archive_name" "$expected_sidecar_name" "VERIFY-DOWNLOAD.txt" | sort
)"
expected_top_level_entries_ordered="$(
  printf '%s\n' "$archive_name" "$expected_sidecar_name" "VERIFY-DOWNLOAD.txt"
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
check_sha256_line_shape "$sidecar_path" "detached checksum sidecar" 1
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
check_top_level_mode() {
  local path="$1"
  local expected_mode="$2"
  local actual_mode
  actual_mode="$(stat -c '%a' "$path")"
  if [[ "$actual_mode" != "$expected_mode" ]]; then
    cat >&2 <<EOF
release artifact top-level entry has unexpected mode: $(basename "$path")
expected: $expected_mode
actual:   $actual_mode
EOF
    exit 1
  fi
}
check_top_level_mode "$archive_path" "644"
check_top_level_mode "$sidecar_path" "644"
check_top_level_mode "$recipe_path" "644"

(
  cd "$ARTIFACT_DIR"
  sha256sum -c "$expected_sidecar_name" >/dev/null
)

archive_listing="$(tar -tzf "$archive_path")"
expected_archive_listing="$(
  printf '%s\n' \
    "bin/modal" \
    "README.txt" \
    "PROVENANCE.txt" \
    "EVIDENCE-BUNDLE.txt" \
    "SHA256SUMS"
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
check_sha256_line_shape "$unpack_dir/SHA256SUMS" "checksum manifest" 4
checksum_entries="$(
  cd "$unpack_dir"
  awk '{ print $2 }' SHA256SUMS | sort
)"
expected_checksum_entries="$(
  printf '%s\n' "EVIDENCE-BUNDLE.txt" "PROVENANCE.txt" "README.txt" "bin/modal" | sort
)"
expected_checksum_entries_ordered="$(
  printf '%s\n' "bin/modal" "README.txt" "PROVENANCE.txt" "EVIDENCE-BUNDLE.txt"
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
checksum_entries_ordered="$(
  awk '{ print $2 }' "$unpack_dir/SHA256SUMS"
)"
if [[ "$checksum_entries_ordered" != "$expected_checksum_entries_ordered" ]]; then
  cat >&2 <<EOF
release artifact checksum manifest has unexpected entry order
expected:
$expected_checksum_entries_ordered
actual:
$checksum_entries_ordered
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
evidence_marker_count="$(
  grep -Fxc "modal replayable evidence bundle" "$unpack_dir/EVIDENCE-BUNDLE.txt" || true
)"
if [[ "$evidence_marker_count" -ne 1 ]]; then
  cat >&2 <<EOF
release artifact evidence manifest has unexpected marker lines
expected:
modal replayable evidence bundle
actual count: $evidence_marker_count
EOF
  exit 1
fi
check_evidence_field() {
  local label="$1"
  local expected="$2"
  local count
  local values
  count="$(
    grep -Ec "^${label}: .+" "$unpack_dir/EVIDENCE-BUNDLE.txt" || true
  )"
  values="$(
    awk -v label="$label" '
      index($0, label ": ") == 1 {
        sub(label ": ", "")
        print
      }
    ' "$unpack_dir/EVIDENCE-BUNDLE.txt"
  )"
  if [[ "$count" -ne 1 || "$values" != "$expected" ]]; then
    cat >&2 <<EOF
release artifact evidence manifest has unexpected $label values
expected:
$expected
actual:
$values
EOF
    exit 1
  fi
}
if ! grep -Fq "source revision:" "$unpack_dir/PROVENANCE.txt"; then
  echo "release artifact provenance is missing source revision" >&2
  exit 1
fi
provenance_marker_count="$(
  grep -Fxc "modal release archive smoke provenance" "$unpack_dir/PROVENANCE.txt" || true
)"
if [[ "$provenance_marker_count" -ne 1 ]]; then
  cat >&2 <<EOF
release artifact provenance has unexpected marker lines
expected:
modal release archive smoke provenance
actual count: $provenance_marker_count
EOF
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
if [[ ! "$provenance_revision" =~ ^[0-9a-f]{7,40}$ ]]; then
  echo "release artifact provenance has unsupported source revision: $provenance_revision" >&2
  exit 1
fi
read_provenance_field() {
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
  printf '%s\n' "$value"
}
provenance_version="$(read_provenance_field "version")"
provenance_profile="$(read_provenance_field "profile")"
provenance_features="$(read_provenance_field "features")"
provenance_help_surface="$(read_provenance_field "help surface")"
provenance_os="$(read_provenance_field "os")"
provenance_arch="$(read_provenance_field "arch")"
version_revision_pattern='@([^)]+)\)'
if [[ "$provenance_version" =~ $version_revision_pattern ]]; then
  provenance_version_revision="${BASH_REMATCH[1]}"
  if [[ "$provenance_version_revision" != "$provenance_revision" ]]; then
    cat >&2 <<EOF
release artifact provenance version revision does not match source revision
version revision: $provenance_version_revision
source revision:  $provenance_revision
EOF
    exit 1
  fi
fi
check_provenance_slug_field() {
  local label="$1"
  local value="$2"
  if [[ ! "$value" =~ ^[a-z0-9._-]+$ ]]; then
    echo "release artifact provenance has unsupported $label value: $value" >&2
    exit 1
  fi
}
check_provenance_slug_field "os" "$provenance_os"
check_provenance_slug_field "arch" "$provenance_arch"
case "$provenance_profile" in
  debug|release)
    ;;
  *)
    echo "release artifact provenance has unsupported profile: $provenance_profile" >&2
    exit 1
    ;;
esac
case "$provenance_help_surface" in
  lean|full)
    ;;
  *)
    echo "release artifact provenance has unsupported help surface: $provenance_help_surface" >&2
    exit 1
    ;;
esac
case "$provenance_features" in
  contract-onboarding|full)
    ;;
  *)
    echo "release artifact provenance has unsupported feature set: $provenance_features" >&2
    exit 1
    ;;
esac
case "$provenance_version" in
  modal\ *)
    provenance_version_slug="$(
      printf '%s' "${provenance_version#modal }" |
        tr '[:upper:]' '[:lower:]' |
        sed -E 's/[^a-z0-9._-]+/-/g; s/^-+//; s/-+$//'
    )"
    ;;
  *)
    echo "release artifact provenance has unexpected version: $provenance_version" >&2
    exit 1
    ;;
esac
if [[ -z "$provenance_version_slug" ]]; then
  echo "release artifact provenance version did not produce a usable archive slug" >&2
  exit 1
fi
expected_archive_name="modal-${provenance_version_slug}-${provenance_os}-${provenance_arch}-${provenance_profile}.tar.gz"
if [[ "$archive_name" != "$expected_archive_name" ]]; then
  cat >&2 <<EOF
release artifact archive name does not match provenance metadata
expected: $expected_archive_name
actual:   $archive_name
EOF
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
check_evidence_field "artifact" "$archive_name"
check_evidence_field "version" "$provenance_version"
check_evidence_field "source revision" "$provenance_revision"
check_evidence_field "profile" "$provenance_profile"
check_evidence_field "features" "$provenance_features"
check_evidence_field "help surface" "$provenance_help_surface"
check_evidence_field "binary" "bin/modal"
check_evidence_field "provenance" "PROVENANCE.txt"
check_evidence_field "checksums" "SHA256SUMS"
check_evidence_field \
  "post-unpack checks" \
  "version, help surface, first-contract smoke when MODALITY_BIN is supplied"
readme_marker_count="$(
  grep -Fxc "modal release archive smoke artifact" "$unpack_dir/README.txt" || true
)"
if [[ "$readme_marker_count" -ne 1 ]]; then
  cat >&2 <<EOF
release artifact README has unexpected marker lines
expected:
modal release archive smoke artifact
actual count: $readme_marker_count
EOF
  exit 1
fi
check_readme_field() {
  local label="$1"
  local expected="$2"
  local count
  local values
  count="$(
    grep -Ec "^${label}: .+" "$unpack_dir/README.txt" || true
  )"
  values="$(
    awk -v label="$label" '
      index($0, label ": ") == 1 {
        sub(label ": ", "")
        print
      }
    ' "$unpack_dir/README.txt"
  )"
  if [[ "$count" -ne 1 || "$values" != "$expected" ]]; then
    cat >&2 <<EOF
release artifact README has unexpected $label values
expected:
$expected
actual:
$values
EOF
    exit 1
  fi
}
check_readme_field "version" "$provenance_version"
check_readme_field "source revision" "$provenance_revision"
check_readme_field "profile" "$provenance_profile"
check_readme_field "features" "$provenance_features"
check_readme_field "help surface" "$provenance_help_surface"
recipe_title_count="$(
  grep -Fxc "modal release archive download verification" "$recipe_path" || true
)"
if [[ "$recipe_title_count" -ne 1 ]]; then
  cat >&2 <<EOF
release artifact verification recipe has unexpected title lines
expected:
modal release archive download verification
actual count: $recipe_title_count
EOF
  exit 1
fi
recipe_first_line="$(sed -n '1p' "$recipe_path")"
if [[ "$recipe_first_line" != "modal release archive download verification" ]]; then
  cat >&2 <<EOF
release artifact verification recipe title must be the first line
expected:
modal release archive download verification
actual:
$recipe_first_line
EOF
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
  ' "$recipe_path"
)"
if [[ "$recipe_expected_top_level_entries" != "$expected_top_level_entries_ordered" ]]; then
  cat >&2 <<EOF
release artifact verification recipe has unexpected downloaded directory entries
expected:
$expected_top_level_entries_ordered
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
if ! grep -Fq "Expected profile:" "$recipe_path"; then
  echo "release artifact verification recipe is missing expected profile label" >&2
  exit 1
fi
recipe_expected_profiles="$(
  awk '
    /^Expected profile:$/ { in_section = 1; next }
    in_section && /^$/ { in_section = 0; next }
    in_section && /^  / { sub(/^  /, ""); print }
  ' "$recipe_path"
)"
if [[ "$recipe_expected_profiles" != "$provenance_profile" ]]; then
  cat >&2 <<EOF
release artifact verification recipe has unexpected profiles
expected:
$provenance_profile
actual:
$recipe_expected_profiles
EOF
  exit 1
fi
if ! grep -Fq "Expected feature set:" "$recipe_path"; then
  echo "release artifact verification recipe is missing expected feature set label" >&2
  exit 1
fi
recipe_expected_feature_sets="$(
  awk '
    /^Expected feature set:$/ { in_section = 1; next }
    in_section && /^$/ { in_section = 0; next }
    in_section && /^  / { sub(/^  /, ""); print }
  ' "$recipe_path"
)"
if [[ "$recipe_expected_feature_sets" != "$provenance_features" ]]; then
  cat >&2 <<EOF
release artifact verification recipe has unexpected feature sets
expected:
$provenance_features
actual:
$recipe_expected_feature_sets
EOF
  exit 1
fi
if ! grep -Fq "Expected help surface:" "$recipe_path"; then
  echo "release artifact verification recipe is missing expected help surface label" >&2
  exit 1
fi
recipe_expected_help_surfaces="$(
  awk '
    /^Expected help surface:$/ { in_section = 1; next }
    in_section && /^$/ { in_section = 0; next }
    in_section && /^  / { sub(/^  /, ""); print }
  ' "$recipe_path"
)"
if [[ "$recipe_expected_help_surfaces" != "$provenance_help_surface" ]]; then
  cat >&2 <<EOF
release artifact verification recipe has unexpected help surfaces
expected:
$provenance_help_surface
actual:
$recipe_expected_help_surfaces
EOF
  exit 1
fi
recipe_sections="$(
  grep -Fnx -e "Artifact:" \
    -e "Expected downloaded directory entries:" \
    -e "Expected source revision:" \
    -e "Expected profile:" \
    -e "Expected feature set:" \
    -e "Expected help surface:" \
    -e "Verify before unpacking or trusting the binary:" \
    "$recipe_path" | cut -d: -f2-
)"
recipe_section_count="$(printf '%s\n' "$recipe_sections" | sed '/^$/d' | wc -l)"
expected_recipe_sections="$(
  printf '%s\n' \
    "Artifact:" \
    "Expected downloaded directory entries:" \
    "Expected source revision:" \
    "Expected profile:" \
    "Expected feature set:" \
    "Expected help surface:" \
    "Verify before unpacking or trusting the binary:"
)"
if [[ "$recipe_section_count" -ne 7 || "$recipe_sections" != "$expected_recipe_sections" ]]; then
  cat >&2 <<EOF
release artifact verification recipe has unexpected section order
expected order:
Artifact:
Expected downloaded directory entries:
Expected source revision:
Expected profile:
Expected feature set:
Expected help surface:
Verify before unpacking or trusting the binary:
actual order:
$recipe_sections
EOF
  exit 1
fi
check_recipe_line() {
  local expected="$1"
  local label="$2"
  local count
  local values
  count="$(grep -Fxc "$expected" "$recipe_path" || true)"
  values="$(grep -Fx "$expected" "$recipe_path" || true)"
  if [[ "$count" -ne 1 ]]; then
    cat >&2 <<EOF
release artifact verification recipe has unexpected $label lines
expected:
$expected
actual:
$values
EOF
    exit 1
  fi
}
check_recipe_line \
  "  sha256sum -c $expected_sidecar_name" \
  "detached checksum command"
expected_verify_command="MODAL_ONBOARDING_ARTIFACT_EXPECT_REV=$provenance_revision tests/cli/check-modal-release-artifact-download.sh /path/to/downloaded-artifact-dir"
check_recipe_line \
  "  $expected_verify_command" \
  "verifier command"
recipe_verify_commands="$(
  awk '
    /^Verify before unpacking or trusting the binary:$/ { in_section = 1; next }
    in_section && /^$/ { in_section = 0; next }
    in_section && /^  / { print }
  ' "$recipe_path"
)"
expected_recipe_verify_commands="$(
  printf '%s\n' \
    "  sha256sum -c $expected_sidecar_name" \
    "  $expected_verify_command"
)"
if [[ "$recipe_verify_commands" != "$expected_recipe_verify_commands" ]]; then
  cat >&2 <<EOF
release artifact verification recipe has unexpected verification commands
expected:
$expected_recipe_verify_commands
actual:
$recipe_verify_commands
EOF
  exit 1
fi
check_recipe_line \
  "Add MODAL_ONBOARDING_ARTIFACT_SMOKE=1 and MODALITY_BIN=/path/to/modality to" \
  "smoke replay environment"
check_recipe_line \
  "run the help-surface and first-contract smokes against the unpacked modal binary." \
  "smoke replay description"
recipe_smoke_replay_trailer="$(
  awk '
    /^Verify before unpacking or trusting the binary:$/ { saw_verify = 1; next }
    saw_verify && /^$/ { in_trailer = 1; next }
    in_trailer { print }
  ' "$recipe_path"
)"
expected_recipe_smoke_replay_trailer="$(
  printf '%s\n' \
    "Add MODAL_ONBOARDING_ARTIFACT_SMOKE=1 and MODALITY_BIN=/path/to/modality to" \
    "run the help-surface and first-contract smokes against the unpacked modal binary."
)"
if [[ "$recipe_smoke_replay_trailer" != "$expected_recipe_smoke_replay_trailer" ]]; then
  cat >&2 <<EOF
release artifact verification recipe has unexpected smoke replay trailer
expected:
$expected_recipe_smoke_replay_trailer
actual:
$recipe_smoke_replay_trailer
EOF
  exit 1
fi
expected_recipe_content="$(cat <<EOF
modal release archive download verification

Artifact:
  $archive_name

Expected downloaded directory entries:
  $archive_name
  $expected_sidecar_name
  VERIFY-DOWNLOAD.txt

Expected source revision:
  $provenance_revision

Expected profile:
  $provenance_profile

Expected feature set:
  $provenance_features

Expected help surface:
  $provenance_help_surface

Verify before unpacking or trusting the binary:
  sha256sum -c $expected_sidecar_name
  $expected_verify_command

Add MODAL_ONBOARDING_ARTIFACT_SMOKE=1 and MODALITY_BIN=/path/to/modality to
run the help-surface and first-contract smokes against the unpacked modal binary.
EOF
)"
actual_recipe_content="$(cat "$recipe_path")"
if [[ "$actual_recipe_content" != "$expected_recipe_content" ]]; then
  cat >&2 <<EOF
release artifact verification recipe is not the canonical emitted recipe
expected:
$expected_recipe_content
actual:
$actual_recipe_content
EOF
  exit 1
fi

if [[ "${MODAL_ONBOARDING_ARTIFACT_SMOKE:-0}" == "1" ]]; then
  ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  unpacked_version="$("$unpack_dir/bin/modal" --version)"
  if [[ "$unpacked_version" != "$provenance_version" ]]; then
    cat >&2 <<EOF
release artifact unpacked modal version does not match provenance
expected: $provenance_version
actual:   $unpacked_version
EOF
    exit 1
  fi
  MODAL_BIN="$unpack_dir/bin/modal" MODAL_HELP_SURFACE="$provenance_help_surface" \
    "$ROOT_DIR/tests/cli/check-modal-help-surface.sh"
  if [[ -x "${MODALITY_BIN:-}" ]]; then
    MODAL_BIN="$unpack_dir/bin/modal" MODALITY_BIN="$MODALITY_BIN" \
      "$ROOT_DIR/tests/cli/run-first-contract-cli-smoke.sh"
  fi
fi

echo "modal release artifact download check passed: $archive_name"
