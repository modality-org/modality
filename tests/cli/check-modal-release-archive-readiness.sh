#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PROFILE="${MODAL_ONBOARDING_PROFILE:-debug}"
HELP_SURFACE="${MODAL_HELP_SURFACE:-lean}"
FEATURES="${MODAL_ONBOARDING_FEATURES:-contract-onboarding}"

case "$PROFILE" in
  debug|release)
    ;;
  *)
    echo "unsupported MODAL_ONBOARDING_PROFILE: $PROFILE" >&2
    echo "expected: debug or release" >&2
    exit 2
    ;;
esac

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
  esac
fi

if [[ ! -f "$MODAL_BIN" || -L "$MODAL_BIN" ]]; then
  cat >&2 <<EOF
release archive readiness check needs a regular non-symlink modal binary
actual: $MODAL_BIN

Build modal from the source checkout under test, or pass a regular binary:
  MODAL_BIN=/path/to/modal $0
EOF
  exit 2
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

capture_command_output_lines() {
  local output_path
  local status
  output_path="$(mktemp)"
  if ! "$@" >"$output_path"; then
    status=$?
    cat "$output_path" >&2 || true
    rm -f "$output_path"
    return "$status"
  fi
  mapfile -t captured_output_lines <"$output_path"
  rm -f "$output_path"
}
captured_output_as_text() {
  printf '%s\n' "${captured_output_lines[@]}"
}

capture_command_output_lines "$MODAL_BIN" --version
if [[ "${#captured_output_lines[@]}" -ne 1 ]]; then
  version_output="$(captured_output_as_text)"
  cat >&2 <<EOF
release archive modal version is not a single line
actual version:
$version_output
EOF
  exit 1
fi
version_output="${captured_output_lines[0]}"
version_revision_marker_count="$(
  grep -Eo '\([^)]*@[^)]+\)' <<<"$version_output" | wc -l || true
)"
version_at_count="$(
  grep -o '@' <<<"$version_output" | wc -l || true
)"
if [[ "$version_revision_marker_count" -gt 1 ]]; then
  cat >&2 <<EOF
release archive modal version has multiple revision markers
actual version:
$version_output
EOF
  exit 1
fi
if [[ "$version_at_count" -ne "$version_revision_marker_count" ]]; then
  cat >&2 <<EOF
release archive modal version has an unsupported revision marker
actual version:
$version_output

Use at most one parenthesized source revision marker ending in @<commit>.
EOF
  exit 1
fi
case "$version_output" in
  modal\ [0-9]*)
    version="${version_output#modal }"
    ;;
  *)
    echo "modal reported unexpected version output: $version_output" >&2
    exit 1
    ;;
esac

os="${MODAL_ONBOARDING_ARCHIVE_OS:-$(uname -s | tr '[:upper:]' '[:lower:]')}"
arch="${MODAL_ONBOARDING_ARCHIVE_ARCH:-$(uname -m)}"
check_archive_slug_field() {
  local label="$1"
  local value="$2"
  if [[ ! "$value" =~ ^[a-z0-9._-]+$ ]]; then
    cat >&2 <<EOF
release archive platform metadata is not archive-safe
field:  $label
actual: $value

Set MODAL_ONBOARDING_ARCHIVE_OS and MODAL_ONBOARDING_ARCHIVE_ARCH only to
lowercase archive-safe platform tokens.
EOF
    exit 1
  fi
}
revisions_match() {
  local expected="$1"
  local actual="$2"
  [[ "$expected" == "$actual" || "$actual" == "$expected"* || "$expected" == "$actual"* ]]
}
check_expected_revision() {
  local name="$1"
  local value="$2"
  if [[ -n "$value" && ! "$value" =~ ^[0-9a-f]{7,40}$ ]]; then
    cat >&2 <<EOF
release archive expected source revision is not a lowercase hex commit token
variable: $name
actual:   $value

Set $name to a full commit hash or an unambiguous Git-style short hash of at
least seven hexadecimal characters.
EOF
    exit 2
  fi
}
check_expected_revision \
  "MODAL_ONBOARDING_ARCHIVE_EXPECT_REV" \
  "${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}"
check_archive_slug_field "os" "$os"
check_archive_slug_field "arch" "$arch"
case "$HELP_SURFACE" in
  lean|full)
    ;;
  *)
    cat >&2 <<EOF
release archive help surface is not supported
actual: $HELP_SURFACE

Set MODAL_HELP_SURFACE only to lean or full before emitting release evidence.
EOF
    exit 1
    ;;
esac
case "$FEATURES" in
  contract-onboarding|full)
    ;;
  *)
    cat >&2 <<EOF
release archive feature set is not supported
actual: $FEATURES

Set MODAL_ONBOARDING_FEATURES only to contract-onboarding or full before
emitting release evidence.
EOF
    exit 1
    ;;
esac
case "$FEATURES:$HELP_SURFACE" in
  contract-onboarding:lean|full:full)
    ;;
  *)
    cat >&2 <<EOF
release archive help surface does not match feature set
features: $FEATURES
help surface: $HELP_SURFACE

Use the lean help surface with contract-onboarding builds and the full help
surface with full builds before emitting replayable release evidence.
EOF
    exit 1
    ;;
esac
version_slug="$(printf '%s' "$version" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9._-]+/-/g; s/^-+//; s/-+$//')"
if [[ -z "$version_slug" ]]; then
  echo "modal version did not produce a usable archive slug: $version_output" >&2
  exit 1
fi
archive_name="modal-${version_slug}-${os}-${arch}-${PROFILE}.tar.gz"
source_revision="${MODAL_ONBOARDING_ARCHIVE_REV:-}"
if [[ -z "$source_revision" ]] && git -C "$ROOT_DIR" rev-parse --verify HEAD >/dev/null 2>&1; then
  source_revision="$(git -C "$ROOT_DIR" rev-parse HEAD)"
fi
version_revision_pattern='\([^)]*@([^)]+)\)'
if [[ -z "$source_revision" && "$version_output" =~ $version_revision_pattern ]]; then
  source_revision="${BASH_REMATCH[1]}"
fi
if [[ -z "$source_revision" ]]; then
  source_revision="unknown"
fi
if [[ ! "$source_revision" =~ ^[0-9a-f]{7,40}$ ]]; then
  cat >&2 <<EOF
release archive source revision is not an archive-safe commit token
actual: $source_revision

Build modal from a Git checkout, or set MODAL_ONBOARDING_ARCHIVE_REV to the
lowercase source commit used for this archive.
EOF
  exit 1
fi
if [[ -n "${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" ]] &&
  ! revisions_match "$MODAL_ONBOARDING_ARCHIVE_EXPECT_REV" "$source_revision"; then
  cat >&2 <<EOF
release archive source revision mismatch
expected: $MODAL_ONBOARDING_ARCHIVE_EXPECT_REV
actual:   $source_revision

Rebuild modal from the expected revision, or unset
MODAL_ONBOARDING_ARCHIVE_EXPECT_REV for a smoke-only archive shape check.
EOF
  exit 1
fi
if [[ "$version_output" =~ $version_revision_pattern ]]; then
  version_revision="${BASH_REMATCH[1]}"
  if [[ ! "$version_revision" =~ ^[0-9a-f]{7,40}$ ]]; then
    cat >&2 <<EOF
release archive modal version revision is not a lowercase hex commit token
version: $version_output
source revision: $source_revision

Build modal from a Git checkout that reports a full commit hash or an
unambiguous Git-style short hash of at least seven hexadecimal characters.
EOF
    exit 1
  fi
  if ! revisions_match "$source_revision" "$version_revision"; then
    cat >&2 <<EOF
release archive modal version revision does not match source revision
version: $version_output
source revision: $source_revision
EOF
    exit 1
  fi
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
NEGATIVE_ARTIFACT_DIR=""
NEGATIVE_STAGE_DIR=""
cleanup() {
  rm -rf "$STAGE_DIR" "$UNPACK_DIR"
  if [[ -n "$NEGATIVE_ARTIFACT_DIR" ]]; then
    rm -rf "$NEGATIVE_ARTIFACT_DIR"
  fi
  if [[ -n "$NEGATIVE_STAGE_DIR" ]]; then
    rm -rf "$NEGATIVE_STAGE_DIR"
  fi
  if [[ "$CLEAN_ARCHIVE_DIR" == "1" ]]; then
    rm -rf "$ARCHIVE_DIR"
  fi
}
trap cleanup EXIT

mkdir -p "$STAGE_DIR/bin"
cp "$MODAL_BIN" "$STAGE_DIR/bin/modal"
chmod 0755 "$STAGE_DIR/bin"
chmod 0755 "$STAGE_DIR/bin/modal"
cat >"$STAGE_DIR/README.txt" <<EOF
modal release archive smoke artifact
version: $version_output
source revision: $source_revision
profile: $PROFILE
features: $FEATURES
help surface: $HELP_SURFACE
EOF
cat >"$STAGE_DIR/PROVENANCE.txt" <<EOF
modal release archive smoke provenance
source revision: $source_revision
version: $version_output
profile: $PROFILE
features: $FEATURES
help surface: $HELP_SURFACE
os: $os
arch: $arch
EOF
cat >"$STAGE_DIR/EVIDENCE-BUNDLE.txt" <<EOF
modal replayable evidence bundle
artifact: $archive_name
version: $version_output
source revision: $source_revision
profile: $PROFILE
features: $FEATURES
help surface: $HELP_SURFACE
binary: bin/modal
provenance: PROVENANCE.txt
checksums: SHA256SUMS
post-unpack checks: version, help surface, same-revision language CLI, first-contract smoke when artifact smoke is enabled
EOF
chmod 0644 "$STAGE_DIR/README.txt" "$STAGE_DIR/PROVENANCE.txt" "$STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
)

ARCHIVE_PATH="$ARCHIVE_DIR/$archive_name"
tar --no-recursion -C "$STAGE_DIR" -czf "$ARCHIVE_PATH" \
  bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
(
  cd "$ARCHIVE_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
  sha256sum -c "$archive_name.sha256" >/dev/null
)
chmod 0644 "$ARCHIVE_PATH" "$ARCHIVE_DIR/$archive_name.sha256"
expected_verify_command="MODAL_ONBOARDING_ARTIFACT_EXPECT_REV=$source_revision tests/cli/check-modal-release-artifact-download.sh /path/to/downloaded-artifact-dir"
cat >"$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" <<EOF
modal release archive download verification

Artifact:
  $archive_name

Expected downloaded directory entries:
  $archive_name
  $archive_name.sha256
  VERIFY-DOWNLOAD.txt

Expected source revision:
  $source_revision

Expected profile:
  $PROFILE

Expected feature set:
  $FEATURES

Expected help surface:
  $HELP_SURFACE

Verify before unpacking or trusting the binary:
  sha256sum -c $archive_name.sha256
  $expected_verify_command

Add MODAL_ONBOARDING_ARTIFACT_SMOKE=1 and MODALITY_BIN=/path/to/modality built
from the same source revision to run version, help-surface, same-revision
language CLI, and first-contract smokes against the unpacked modal binary.
EOF
chmod 0644 "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt"
MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
  "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$ARCHIVE_DIR" >/dev/null
if [[ -n "${MODALITY_BIN:-}" && ! -x "$MODALITY_BIN" ]]; then
  cat >&2 <<EOF
release archive smoke replay needs an executable MODALITY_BIN
actual: $MODALITY_BIN

Unset MODALITY_BIN for archive-only verification, or set it to
/path/to/modality built from the same source revision to run the first-contract
smoke against the unpacked modal binary.
EOF
  exit 2
fi
if [[ -x "${MODALITY_BIN:-}" ]]; then
  MODAL_ONBOARDING_ARTIFACT_SMOKE=1 \
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
  MODALITY_BIN="$MODALITY_BIN" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$ARCHIVE_DIR" >/dev/null
fi
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_SMOKE=1 \
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    env -u MODALITY_BIN \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$ARCHIVE_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted smoke replay without MODALITY_BIN" >&2
  exit 1
}
if ! grep -Fq "release artifact smoke replay requires MODALITY_BIN" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the missing smoke modality binary for the wrong reason
expected: release artifact smoke replay requires MODALITY_BIN
actual:
$negative_output
EOF
  exit 1
fi
NONEXEC_MODALITY="$(mktemp)"
cat >"$NONEXEC_MODALITY" <<'EOF'
#!/usr/bin/env bash
echo "modality 0.0.0 (@0000000)"
EOF
chmod 0644 "$NONEXEC_MODALITY"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_SMOKE=1 \
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
  MODALITY_BIN="$NONEXEC_MODALITY" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$ARCHIVE_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted smoke replay with non-executable MODALITY_BIN" >&2
  exit 1
}
if ! grep -Fq "release artifact smoke replay needs an executable MODALITY_BIN" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the non-executable smoke modality binary for the wrong reason
expected: release artifact smoke replay needs an executable MODALITY_BIN
actual:
$negative_output
EOF
  exit 1
fi
SYMLINK_MODALITY_TARGET="$(mktemp)"
SYMLINK_MODALITY="$(mktemp -u)"
cat >"$SYMLINK_MODALITY_TARGET" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "--version" ]]; then
  echo "modality 0.0.0 (@$source_revision)"
  exit 0
fi
echo "symlinked modality test binary should only be asked for --version" >&2
exit 1
EOF
chmod 0755 "$SYMLINK_MODALITY_TARGET"
ln -s "$SYMLINK_MODALITY_TARGET" "$SYMLINK_MODALITY"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_SMOKE=1 \
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
  MODALITY_BIN="$SYMLINK_MODALITY" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$ARCHIVE_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted smoke replay with symlinked MODALITY_BIN" >&2
  exit 1
}
if ! grep -Fq "release artifact smoke replay needs a regular non-symlink MODALITY_BIN" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the symlinked smoke modality binary for the wrong reason
expected: release artifact smoke replay needs a regular non-symlink MODALITY_BIN
actual:
$negative_output
EOF
  exit 1
fi
FAKE_STALE_MODALITY="$(mktemp)"
cat >"$FAKE_STALE_MODALITY" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "--version" ]]; then
  echo "modality 0.0.0 (stale@deadbee)"
  exit 0
fi
echo "stale modality test binary should only be asked for --version" >&2
exit 1
EOF
chmod 0755 "$FAKE_STALE_MODALITY"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_SMOKE=1 \
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
  MODALITY_BIN="$FAKE_STALE_MODALITY" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$ARCHIVE_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a stale modality smoke binary" >&2
  exit 1
}
if ! grep -Fq "release artifact smoke modality version does not match provenance source revision" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the stale modality smoke binary for the wrong reason
expected: release artifact smoke modality version does not match provenance source revision
actual:
$negative_output
EOF
  exit 1
fi
FAKE_SHORT_REV_MODALITY="$(mktemp)"
cat >"$FAKE_SHORT_REV_MODALITY" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "--version" ]]; then
  echo "modality 0.0.0 (@${source_revision:0:6})"
  exit 0
fi
echo "short-revision modality test binary should only be asked for --version" >&2
exit 1
EOF
chmod 0755 "$FAKE_SHORT_REV_MODALITY"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_SMOKE=1 \
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
  MODALITY_BIN="$FAKE_SHORT_REV_MODALITY" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$ARCHIVE_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a modality smoke binary with a too-short revision" >&2
  exit 1
}
if ! grep -Fq "release artifact smoke modality version revision is not a lowercase hex commit token" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the too-short modality smoke revision for the wrong reason
expected: release artifact smoke modality version revision is not a lowercase hex commit token
actual:
$negative_output
EOF
  exit 1
fi
FAKE_NO_REV_MODALITY="$(mktemp)"
cat >"$FAKE_NO_REV_MODALITY" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "--version" ]]; then
  echo "modality 0.0.0"
  exit 0
fi
echo "no-revision modality test binary should only be asked for --version" >&2
exit 1
EOF
chmod 0755 "$FAKE_NO_REV_MODALITY"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_SMOKE=1 \
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
  MODALITY_BIN="$FAKE_NO_REV_MODALITY" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$ARCHIVE_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a modality smoke binary without a source revision" >&2
  exit 1
}
if ! grep -Fq "release artifact smoke modality version does not include a source revision" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the no-revision modality smoke binary for the wrong reason
expected: release artifact smoke modality version does not include a source revision
actual:
$negative_output
EOF
  exit 1
fi
FAKE_MULTILINE_MODALITY="$(mktemp)"
cat >"$FAKE_MULTILINE_MODALITY" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "--version" ]]; then
  printf 'modality 0.0.0 (@$source_revision)\\nstale extra revision note\\n'
  exit 0
fi
echo "multiline modality test binary should only be asked for --version" >&2
exit 1
EOF
chmod 0755 "$FAKE_MULTILINE_MODALITY"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_SMOKE=1 \
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
  MODALITY_BIN="$FAKE_MULTILINE_MODALITY" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$ARCHIVE_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a modality smoke binary with multi-line version output" >&2
  exit 1
}
if ! grep -Fq "release artifact smoke modality version is not a single line" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the multi-line modality smoke version for the wrong reason
expected: release artifact smoke modality version is not a single line
actual:
$negative_output
EOF
  exit 1
fi
FAKE_EXTRA_REV_MODALITY="$(mktemp)"
cat >"$FAKE_EXTRA_REV_MODALITY" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "--version" ]]; then
  echo "modality 0.0.0 (@$source_revision) stale (@deadbee)"
  exit 0
fi
echo "extra-revision modality test binary should only be asked for --version" >&2
exit 1
EOF
chmod 0755 "$FAKE_EXTRA_REV_MODALITY"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_SMOKE=1 \
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
  MODALITY_BIN="$FAKE_EXTRA_REV_MODALITY" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$ARCHIVE_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a modality smoke binary with multiple revision markers" >&2
  exit 1
}
if ! grep -Fq "release artifact smoke modality version has multiple revision markers" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the extra-revision modality smoke version for the wrong reason
expected: release artifact smoke modality version has multiple revision markers
actual:
$negative_output
EOF
    exit 1
fi
FAKE_BARE_REV_MODALITY="$(mktemp)"
cat >"$FAKE_BARE_REV_MODALITY" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "--version" ]]; then
  echo "modality 0.0.0 (@$source_revision) stale @deadbee"
  exit 0
fi
echo "bare-revision modality test binary should only be asked for --version" >&2
exit 1
EOF
chmod 0755 "$FAKE_BARE_REV_MODALITY"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_SMOKE=1 \
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
  MODALITY_BIN="$FAKE_BARE_REV_MODALITY" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$ARCHIVE_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a modality smoke binary with a bare revision marker" >&2
  exit 1
}
if ! grep -Fq "release artifact smoke modality version has an unsupported revision marker" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the bare-revision modality smoke version for the wrong reason
expected: release artifact smoke modality version has an unsupported revision marker
actual:
$negative_output
EOF
  exit 1
fi
FAKE_PREFIX_MODALITY="$(mktemp)"
cat >"$FAKE_PREFIX_MODALITY" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "--version" ]]; then
  echo "not-modality 0.0.0 (@$source_revision)"
  exit 0
fi
echo "wrong-prefix modality test binary should only be asked for --version" >&2
exit 1
EOF
chmod 0755 "$FAKE_PREFIX_MODALITY"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_SMOKE=1 \
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
  MODALITY_BIN="$FAKE_PREFIX_MODALITY" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$ARCHIVE_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a smoke binary with a non-modality version prefix" >&2
  exit 1
}
if ! grep -Fq "release artifact smoke modality binary reported an unexpected version prefix" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the non-modality smoke version prefix for the wrong reason
expected: release artifact smoke modality binary reported an unexpected version prefix
actual:
$negative_output
EOF
  exit 1
fi
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_SMOKE=yes \
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$ARCHIVE_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted an ambiguous smoke flag" >&2
  exit 1
}
if ! grep -Fq "release artifact smoke flag has unsupported value" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the ambiguous smoke flag for the wrong reason
expected: release artifact smoke flag has unsupported value
actual:
$negative_output
EOF
  exit 1
fi
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_SMOKE=0 \
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$ARCHIVE_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted an explicit disabled smoke flag" >&2
  exit 1
}
if ! grep -Fq "release artifact smoke flag has unsupported value" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the explicit disabled smoke flag for the wrong reason
expected: release artifact smoke flag has unsupported value
actual:
$negative_output
EOF
  exit 1
fi
short_expected_revision="${source_revision:0:6}"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="$short_expected_revision" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$ARCHIVE_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a too-short expected revision" >&2
  exit 1
}
if ! grep -Fq "release artifact expected source revision is not a lowercase hex commit token" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the too-short expected revision for the wrong reason
expected: release artifact expected source revision is not a lowercase hex commit token
actual:
$negative_output
EOF
  exit 1
fi
NONEXEC_MODAL="$(mktemp)"
cat >"$NONEXEC_MODAL" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "--version" ]]; then
  echo "modal 0.0.0 (@$source_revision)"
  exit 0
fi
echo "non-executable modal test binary should only be asked for --version" >&2
exit 1
EOF
chmod 0644 "$NONEXEC_MODAL"
negative_output="$(
  MODAL_BIN="$NONEXEC_MODAL" \
    "$ROOT_DIR/tests/cli/check-modal-release-archive-readiness.sh" 2>&1
)" && {
  echo "release archive producer accepted a non-executable modal binary" >&2
  exit 1
}
if ! grep -Fq "release archive readiness check needs a built modal binary" <<<"$negative_output"; then
  cat >&2 <<EOF
release archive producer rejected the non-executable modal binary for the wrong reason
expected: release archive readiness check needs a built modal binary
actual:
$negative_output
EOF
  exit 1
fi
SYMLINK_MODAL_TARGET="$(mktemp)"
SYMLINK_MODAL="$(mktemp -u)"
cat >"$SYMLINK_MODAL_TARGET" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "--version" ]]; then
  echo "modal 0.0.0 (@$source_revision)"
  exit 0
fi
echo "symlinked modal test binary should only be asked for --version" >&2
exit 1
EOF
chmod 0755 "$SYMLINK_MODAL_TARGET"
ln -s "$SYMLINK_MODAL_TARGET" "$SYMLINK_MODAL"
negative_output="$(
  MODAL_BIN="$SYMLINK_MODAL" \
    "$ROOT_DIR/tests/cli/check-modal-release-archive-readiness.sh" 2>&1
)" && {
  echo "release archive producer accepted a symlinked modal binary" >&2
  exit 1
}
if ! grep -Fq "release archive readiness check needs a regular non-symlink modal binary" <<<"$negative_output"; then
  cat >&2 <<EOF
release archive producer rejected the symlinked modal binary for the wrong reason
expected: release archive readiness check needs a regular non-symlink modal binary
actual:
$negative_output
EOF
  exit 1
fi
FAKE_TRAILING_BLANK_MODAL="$(mktemp)"
cat >"$FAKE_TRAILING_BLANK_MODAL" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "--version" ]]; then
  printf '%s\\n\\n' "modal 0.0.0 (@$source_revision)"
  exit 0
fi
echo "trailing-blank modal test binary should only be asked for --version" >&2
exit 1
EOF
chmod 0755 "$FAKE_TRAILING_BLANK_MODAL"
negative_output="$(
  MODAL_BIN="$FAKE_TRAILING_BLANK_MODAL" \
    "$ROOT_DIR/tests/cli/check-modal-release-archive-readiness.sh" 2>&1
)" && {
  echo "release archive producer accepted modal version output with a trailing blank line" >&2
  exit 1
}
if ! grep -Fq "release archive modal version is not a single line" <<<"$negative_output"; then
  cat >&2 <<EOF
release archive producer rejected the trailing-blank modal version for the wrong reason
expected: release archive modal version is not a single line
actual:
$negative_output
EOF
  exit 1
fi
FAKE_EXTRA_REV_MODAL="$(mktemp)"
cat >"$FAKE_EXTRA_REV_MODAL" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "--version" ]]; then
  echo "modal 0.0.0 (@$source_revision) stale (@deadbee)"
  exit 0
fi
echo "extra-revision modal test binary should only be asked for --version" >&2
exit 1
EOF
chmod 0755 "$FAKE_EXTRA_REV_MODAL"
negative_output="$(
  MODAL_BIN="$FAKE_EXTRA_REV_MODAL" \
    "$ROOT_DIR/tests/cli/check-modal-release-archive-readiness.sh" 2>&1
)" && {
  echo "release archive producer accepted modal version output with multiple revision markers" >&2
  exit 1
}
if ! grep -Fq "release archive modal version has multiple revision markers" <<<"$negative_output"; then
  cat >&2 <<EOF
release archive producer rejected the extra-revision modal version for the wrong reason
expected: release archive modal version has multiple revision markers
actual:
$negative_output
EOF
  exit 1
fi
FAKE_BARE_REV_MODAL="$(mktemp)"
cat >"$FAKE_BARE_REV_MODAL" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "--version" ]]; then
  echo "modal 0.0.0 (@$source_revision) stale @deadbee"
  exit 0
fi
echo "bare-revision modal test binary should only be asked for --version" >&2
exit 1
EOF
chmod 0755 "$FAKE_BARE_REV_MODAL"
negative_output="$(
  MODAL_BIN="$FAKE_BARE_REV_MODAL" \
    "$ROOT_DIR/tests/cli/check-modal-release-archive-readiness.sh" 2>&1
)" && {
  echo "release archive producer accepted modal version output with a bare revision marker" >&2
  exit 1
}
if ! grep -Fq "release archive modal version has an unsupported revision marker" <<<"$negative_output"; then
  cat >&2 <<EOF
release archive producer rejected the bare-revision modal version for the wrong reason
expected: release archive modal version has an unsupported revision marker
actual:
$negative_output
EOF
  exit 1
fi
FAKE_SHORT_REV_MODAL="$(mktemp)"
cat >"$FAKE_SHORT_REV_MODAL" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "--version" ]]; then
  echo "modal 0.0.0 (@${source_revision:0:6})"
  exit 0
fi
echo "short-revision modal test binary should only be asked for --version" >&2
exit 1
EOF
chmod 0755 "$FAKE_SHORT_REV_MODAL"
negative_output="$(
  MODAL_BIN="$FAKE_SHORT_REV_MODAL" \
  MODAL_ONBOARDING_ARCHIVE_REV="$source_revision" \
    "$ROOT_DIR/tests/cli/check-modal-release-archive-readiness.sh" 2>&1
)" && {
  echo "release archive producer accepted modal version output with a too-short revision" >&2
  exit 1
}
if ! grep -Fq "release archive modal version revision is not a lowercase hex commit token" <<<"$negative_output"; then
  cat >&2 <<EOF
release archive producer rejected the too-short modal version revision for the wrong reason
expected: release archive modal version revision is not a lowercase hex commit token
actual:
$negative_output
EOF
  exit 1
fi
FAKE_STALE_REV_MODAL="$(mktemp)"
cat >"$FAKE_STALE_REV_MODAL" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "--version" ]]; then
  echo "modal 0.0.0 (@deadbee)"
  exit 0
fi
echo "stale-revision modal test binary should only be asked for --version" >&2
exit 1
EOF
chmod 0755 "$FAKE_STALE_REV_MODAL"
negative_output="$(
  MODAL_BIN="$FAKE_STALE_REV_MODAL" \
  MODAL_ONBOARDING_ARCHIVE_REV="$source_revision" \
    "$ROOT_DIR/tests/cli/check-modal-release-archive-readiness.sh" 2>&1
)" && {
  echo "release archive producer accepted modal version output with a stale revision marker" >&2
  exit 1
}
if ! grep -Fq "release archive modal version revision does not match source revision" <<<"$negative_output"; then
  cat >&2 <<EOF
release archive producer rejected the stale modal version revision for the wrong reason
expected: release archive modal version revision does not match source revision
actual:
$negative_output
EOF
  exit 1
fi
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
mkdir "$NEGATIVE_ARTIFACT_DIR/unexpected"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted an unexpected top-level directory" >&2
  exit 1
}
if ! grep -Fq "release artifact download directory has unexpected top-level entries" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the malformed directory for the wrong reason
expected: release artifact download directory has unexpected top-level entries
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
FAKE_IMPLICIT_STALE_REV_MODAL="$(mktemp)"
cat >"$FAKE_IMPLICIT_STALE_REV_MODAL" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "--version" ]]; then
  echo "modal 0.0.0 (@deadbee)"
  exit 0
fi
echo "implicit-stale-revision modal test binary should only be asked for --version" >&2
exit 1
EOF
chmod 0755 "$FAKE_IMPLICIT_STALE_REV_MODAL"
negative_output="$(
  MODAL_BIN="$FAKE_IMPLICIT_STALE_REV_MODAL" \
    "$ROOT_DIR/tests/cli/check-modal-release-archive-readiness.sh" 2>&1
)" && {
  echo "release archive producer accepted modal version output with a stale implicit revision marker" >&2
  exit 1
}
if ! grep -Fq "release archive modal version revision does not match source revision" <<<"$negative_output"; then
  cat >&2 <<EOF
release archive producer rejected the stale implicit modal version revision for the wrong reason
expected: release archive modal version revision does not match source revision
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
chmod 0600 "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a top-level recipe with the wrong mode" >&2
  exit 1
}
if ! grep -Fq "release artifact top-level entry has unexpected mode: VERIFY-DOWNLOAD.txt" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the bad-mode recipe for the wrong reason
expected: release artifact top-level entry has unexpected mode: VERIFY-DOWNLOAD.txt
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
mkdir "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a directory verification recipe" >&2
  exit 1
}
if ! grep -Fq "release artifact top-level entry must be a regular non-symlink file: VERIFY-DOWNLOAD.txt" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the directory recipe for the wrong reason
expected: release artifact top-level entry must be a regular non-symlink file: VERIFY-DOWNLOAD.txt
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
mkdir "$NEGATIVE_ARTIFACT_DIR/$archive_name"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a directory archive payload" >&2
  exit 1
}
if ! grep -Fq "release artifact top-level entry must be a regular non-symlink file: $archive_name" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the directory archive for the wrong reason
expected: release artifact top-level entry must be a regular non-symlink file: $archive_name
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
stale_sidecar_name="modal-stale-${os}-${arch}-${PROFILE}.tar.gz.sha256"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/$stale_sidecar_name"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a checksum sidecar with a stale name" >&2
  exit 1
}
if ! grep -Fq "release artifact checksum sidecar does not match archive name" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the stale-name checksum sidecar for the wrong reason
expected: release artifact checksum sidecar does not match archive name
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
mkdir "$NEGATIVE_ARTIFACT_DIR/$archive_name.sha256"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a directory checksum sidecar" >&2
  exit 1
}
if ! grep -Fq "release artifact top-level entry must be a regular non-symlink file: $archive_name.sha256" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the directory checksum sidecar for the wrong reason
expected: release artifact top-level entry must be a regular non-symlink file: $archive_name.sha256
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
ln -s "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a symlinked verification recipe" >&2
  exit 1
}
if ! grep -Fq "release artifact top-level entry must be a regular non-symlink file: VERIFY-DOWNLOAD.txt" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the symlinked recipe for the wrong reason
expected: release artifact top-level entry must be a regular non-symlink file: VERIFY-DOWNLOAD.txt
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
ln -s "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/$archive_name"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a symlinked archive payload" >&2
  exit 1
}
if ! grep -Fq "release artifact top-level entry must be a regular non-symlink file: $archive_name" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the symlinked archive for the wrong reason
expected: release artifact top-level entry must be a regular non-symlink file: $archive_name
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
ln -s "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/$archive_name.sha256"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a symlinked checksum sidecar" >&2
  exit 1
}
if ! grep -Fq "release artifact top-level entry must be a regular non-symlink file: $archive_name.sha256" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the symlinked checksum sidecar for the wrong reason
expected: release artifact top-level entry must be a regular non-symlink file: $archive_name.sha256
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
chmod 0600 "$NEGATIVE_ARTIFACT_DIR/$archive_name"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a top-level archive with the wrong mode" >&2
  exit 1
}
if ! grep -Fq "release artifact top-level entry has unexpected mode: $archive_name" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the bad-mode archive for the wrong reason
expected: release artifact top-level entry has unexpected mode: $archive_name
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
chmod 0600 "$NEGATIVE_ARTIFACT_DIR/$archive_name.sha256"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a top-level checksum sidecar with the wrong mode" >&2
  exit 1
}
if ! grep -Fq "release artifact top-level entry has unexpected mode: $archive_name.sha256" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the bad-mode checksum sidecar for the wrong reason
expected: release artifact top-level entry has unexpected mode: $archive_name.sha256
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
ln -s /bin/sh "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a symlinked unpacked modal payload" >&2
  exit 1
}
if ! grep -Fq "release artifact unpacked entry must be a regular non-symlink file: bin/modal" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the symlinked unpacked payload for the wrong reason
expected: release artifact unpacked entry must be a regular non-symlink file: bin/modal
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/real-bin"
ln -s real-bin "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/real-bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/real-bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a symlinked unpacked bin directory" >&2
  exit 1
}
if ! grep -Fq "release artifact archive has unexpected entries" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the symlinked bin directory for the wrong reason
expected: release artifact archive has unexpected entries
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0600 "$NEGATIVE_STAGE_DIR/README.txt"
chmod 0644 "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted an unpacked README payload with the wrong mode" >&2
  exit 1
}
if ! grep -Fq "release artifact unpacked entry has unexpected mode: README.txt" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the bad-mode payload for the wrong reason
expected: release artifact unpacked entry has unexpected mode: README.txt
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0700 "$NEGATIVE_STAGE_DIR/bin"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted an unpacked bin directory with the wrong mode" >&2
  exit 1
}
if ! grep -Fq "release artifact unpacked entry has unexpected mode: bin" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the bad-mode bin directory for the wrong reason
expected: release artifact unpacked entry has unexpected mode: bin
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    README.txt bin bin/modal PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a tarball with shuffled archive members" >&2
  exit 1
}
if ! grep -Fq "release artifact archive has unexpected entries" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the shuffled archive members for the wrong reason
expected: release artifact archive has unexpected entries
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum README.txt bin/modal PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a checksum manifest with shuffled entries" >&2
  exit 1
}
if ! grep -Fq "release artifact checksum manifest has unexpected entry order" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the shuffled checksum manifest for the wrong reason
expected: release artifact checksum manifest has unexpected entry order
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cat "$ARCHIVE_DIR/$archive_name.sha256" >>"$NEGATIVE_ARTIFACT_DIR/$archive_name.sha256"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a duplicated detached checksum sidecar entry" >&2
  exit 1
}
if ! grep -Fq "release artifact detached checksum sidecar has unexpected checksum lines" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the duplicated detached checksum sidecar for the wrong reason
expected: release artifact detached checksum sidecar has unexpected checksum lines
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
perl -pi -e 's/^[0-9a-f]/A/' "$NEGATIVE_ARTIFACT_DIR/$archive_name.sha256"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a malformed detached checksum sidecar line" >&2
  exit 1
}
if ! grep -Fq "release artifact detached checksum sidecar has unexpected checksum lines" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the malformed detached checksum sidecar for the wrong reason
expected: release artifact detached checksum sidecar has unexpected checksum lines
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
wrong_archive_name="modal-stale-${os}-${arch}-${PROFILE}.tar.gz"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
sed "s/artifact: $archive_name/artifact: $wrong_archive_name/" \
  "$STAGE_DIR/EVIDENCE-BUNDLE.txt" >"$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$wrong_archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$wrong_archive_name" >"$wrong_archive_name.sha256"
)
sed "s/$archive_name/$wrong_archive_name/g" "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" \
  >"$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted an archive name that disagrees with provenance" >&2
  exit 1
}
if ! grep -Fq "release artifact archive name does not match provenance metadata" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the renamed archive for the wrong reason
expected: release artifact archive name does not match provenance metadata
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
sed -i '/^Artifact:$/,+2d' "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe without artifact metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe is missing artifact label" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the missing-artifact recipe for the wrong reason
expected: release artifact verification recipe is missing artifact label
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
sed -i '/^  '"$archive_name"'$/a\  stale-or-ambiguous.tar.gz' "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe with ambiguous artifact names" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe has unexpected artifacts" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the ambiguous recipe artifacts for the wrong reason
expected: release artifact verification recipe has unexpected artifacts
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
cat >>"$NEGATIVE_STAGE_DIR/PROVENANCE.txt" <<EOF
source revision: stale-or-ambiguous
EOF
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted ambiguous provenance source revisions" >&2
  exit 1
}
if ! grep -Fq "release artifact provenance must name exactly one source revision" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected ambiguous provenance for the wrong reason
expected: release artifact provenance must name exactly one source revision
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
sed "s/^source revision: $source_revision\$/source revision: unknown/" \
  "$STAGE_DIR/PROVENANCE.txt" >"$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
sed "s/^source revision: $source_revision\$/source revision: unknown/" \
  "$STAGE_DIR/EVIDENCE-BUNDLE.txt" >"$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
sed "s/^  $source_revision\$/  unknown/" \
  "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" >"$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted unsupported provenance source revision metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact provenance has unsupported source revision: unknown" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected unsupported source revision metadata for the wrong reason
expected: release artifact provenance has unsupported source revision: unknown
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
grep -Fv "help surface:" "$STAGE_DIR/PROVENANCE.txt" >"$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted stale provenance without help-surface metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact provenance must name exactly one help surface" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected stale provenance for the wrong reason
expected: release artifact provenance must name exactly one help surface
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
grep -Fv "help surface:" "$STAGE_DIR/README.txt" >"$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted stale README metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact README has unexpected help surface values" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected stale README metadata for the wrong reason
expected: release artifact README has unexpected help surface values
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
sed "s/^version: modal /version: modal stale-/" \
  "$STAGE_DIR/README.txt" >"$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted stale README version metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact README has unexpected version values" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected stale README version metadata for the wrong reason
expected: release artifact README has unexpected version values
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
sed "s/^source revision: $source_revision\$/source revision: stale-or-ambiguous/" \
  "$STAGE_DIR/README.txt" >"$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted stale README source revision metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact README has unexpected source revision values" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected stale README source revision metadata for the wrong reason
expected: release artifact README has unexpected source revision values
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
sed "s/^features: $FEATURES\$/features: stale-or-ambiguous/" \
  "$STAGE_DIR/README.txt" >"$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted stale README feature metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact README has unexpected features values" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected stale README feature metadata for the wrong reason
expected: release artifact README has unexpected features values
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cat >>"$NEGATIVE_STAGE_DIR/README.txt" <<EOF
help surface: stale-full
EOF
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted ambiguous README metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact README has unexpected help surface values" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected ambiguous README metadata for the wrong reason
expected: release artifact README has unexpected help surface values
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cat "$STAGE_DIR/README.txt" >"$NEGATIVE_STAGE_DIR/README.txt"
cat >>"$NEGATIVE_STAGE_DIR/README.txt" <<EOF
modal release archive smoke artifact
EOF
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted ambiguous README marker metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact README has unexpected marker lines" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected ambiguous README marker metadata for the wrong reason
expected: release artifact README has unexpected marker lines
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
sed "s/^artifact: $archive_name\$/artifact: modal-stale-artifact.tar.gz/" \
  "$STAGE_DIR/EVIDENCE-BUNDLE.txt" >"$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted stale evidence manifest artifact metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact evidence manifest has unexpected artifact values" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected stale evidence artifact metadata for the wrong reason
expected: release artifact evidence manifest has unexpected artifact values
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
grep -Fv "checksums: SHA256SUMS" "$STAGE_DIR/EVIDENCE-BUNDLE.txt" >"$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted an evidence manifest without checksum provenance" >&2
  exit 1
}
if ! grep -Fq "release artifact evidence manifest has unexpected checksums values" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the stale evidence manifest for the wrong reason
expected: release artifact evidence manifest has unexpected checksums values
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
sed "s/^version: modal /version: modal stale-/" \
  "$STAGE_DIR/EVIDENCE-BUNDLE.txt" >"$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted stale evidence manifest version metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact evidence manifest has unexpected version values" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected stale evidence version metadata for the wrong reason
expected: release artifact evidence manifest has unexpected version values
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
sed "s/^features: $FEATURES\$/features: stale-features/" \
  "$STAGE_DIR/EVIDENCE-BUNDLE.txt" >"$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted stale evidence manifest feature metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact evidence manifest has unexpected features values" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected stale evidence feature metadata for the wrong reason
expected: release artifact evidence manifest has unexpected features values
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
sed "s/^help surface: $HELP_SURFACE\$/help surface: stale-full/" \
  "$STAGE_DIR/EVIDENCE-BUNDLE.txt" >"$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted stale evidence manifest help-surface metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact evidence manifest has unexpected help surface values" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected stale evidence help-surface metadata for the wrong reason
expected: release artifact evidence manifest has unexpected help surface values
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cat "$STAGE_DIR/EVIDENCE-BUNDLE.txt" >"$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
cat >>"$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt" <<EOF
binary: bin/stale-modal
EOF
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted ambiguous evidence manifest metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact evidence manifest has unexpected binary values" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected ambiguous evidence metadata for the wrong reason
expected: release artifact evidence manifest has unexpected binary values
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cat "$STAGE_DIR/EVIDENCE-BUNDLE.txt" >"$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
cat >>"$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt" <<EOF
modal replayable evidence bundle
EOF
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted ambiguous evidence manifest marker metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact evidence manifest has unexpected marker lines" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected ambiguous evidence marker metadata for the wrong reason
expected: release artifact evidence manifest has unexpected marker lines
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
unsupported_profile="smoke"
unsupported_profile_archive_name="modal-${version_slug}-${os}-${arch}-${unsupported_profile}.tar.gz"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
sed "s/^profile: $PROFILE\$/profile: $unsupported_profile/" \
  "$STAGE_DIR/PROVENANCE.txt" >"$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
sed "s/artifact: $archive_name/artifact: $unsupported_profile_archive_name/" \
  "$STAGE_DIR/EVIDENCE-BUNDLE.txt" >"$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$unsupported_profile_archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$unsupported_profile_archive_name" >"$unsupported_profile_archive_name.sha256"
)
sed "s/$archive_name/$unsupported_profile_archive_name/g" "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" \
  >"$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted an unsupported provenance profile" >&2
  exit 1
}
if ! grep -Fq "release artifact provenance has unsupported profile: $unsupported_profile" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the unsupported-profile artifact for the wrong reason
expected: release artifact provenance has unsupported profile: $unsupported_profile
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
unsupported_help_surface="experimental"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
sed "s/^help surface: $HELP_SURFACE\$/help surface: $unsupported_help_surface/" \
  "$STAGE_DIR/README.txt" >"$NEGATIVE_STAGE_DIR/README.txt"
sed "s/^help surface: $HELP_SURFACE\$/help surface: $unsupported_help_surface/" \
  "$STAGE_DIR/PROVENANCE.txt" >"$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
sed "s/^  $HELP_SURFACE\$/  $unsupported_help_surface/" \
  "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" >"$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted an unsupported provenance help surface" >&2
  exit 1
}
if ! grep -Fq "release artifact provenance has unsupported help surface: $unsupported_help_surface" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the unsupported-help-surface artifact for the wrong reason
expected: release artifact provenance has unsupported help surface: $unsupported_help_surface
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
unsupported_features="experimental-features"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
sed "s/^features: $FEATURES\$/features: $unsupported_features/" \
  "$STAGE_DIR/PROVENANCE.txt" >"$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted an unsupported provenance feature set" >&2
  exit 1
}
if ! grep -Fq "release artifact provenance has unsupported feature set: $unsupported_features" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the unsupported-feature artifact for the wrong reason
expected: release artifact provenance has unsupported feature set: $unsupported_features
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
if [[ "$HELP_SURFACE" == "lean" ]]; then
  mismatched_help_surface="full"
else
  mismatched_help_surface="lean"
fi
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
sed "s/^help surface: $HELP_SURFACE\$/help surface: $mismatched_help_surface/" \
  "$STAGE_DIR/README.txt" >"$NEGATIVE_STAGE_DIR/README.txt"
sed "s/^help surface: $HELP_SURFACE\$/help surface: $mismatched_help_surface/" \
  "$STAGE_DIR/PROVENANCE.txt" >"$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
sed "s/^help surface: $HELP_SURFACE\$/help surface: $mismatched_help_surface/" \
  "$STAGE_DIR/EVIDENCE-BUNDLE.txt" >"$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
sed "s/^  $HELP_SURFACE\$/  $mismatched_help_surface/" \
  "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" >"$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted mismatched feature/help-surface metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact help surface does not match feature set" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the mismatched feature/help artifact for the wrong reason
expected: release artifact help surface does not match feature set
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
bad_shape_version_revision="${source_revision}x"
if [[ "$version_output" =~ $version_revision_pattern ]]; then
  bad_shape_version_output="$(
    printf '%s' "$version_output" | sed -E "s/@[^)]*\\)/@$bad_shape_version_revision)/"
  )"
else
  bad_shape_version_output="$version_output (source@$bad_shape_version_revision)"
fi
bad_shape_version_slug="$(
  printf '%s' "${bad_shape_version_output#modal }" |
    tr '[:upper:]' '[:lower:]' |
    sed -E 's/[^a-z0-9._-]+/-/g; s/^-+//; s/-+$//'
)"
bad_shape_version_archive_name="modal-${bad_shape_version_slug}-${os}-${arch}-${PROFILE}.tar.gz"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
sed "s/^version: .*$/version: $bad_shape_version_output/" \
  "$STAGE_DIR/README.txt" >"$NEGATIVE_STAGE_DIR/README.txt"
sed "s/^version: .*$/version: $bad_shape_version_output/" \
  "$STAGE_DIR/PROVENANCE.txt" >"$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
sed "s/artifact: $archive_name/artifact: $bad_shape_version_archive_name/" \
  "$STAGE_DIR/EVIDENCE-BUNDLE.txt" >"$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$bad_shape_version_archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$bad_shape_version_archive_name" >"$bad_shape_version_archive_name.sha256"
)
sed "s/$archive_name/$bad_shape_version_archive_name/g" "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" \
  >"$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted provenance version metadata with a malformed revision token" >&2
  exit 1
}
if ! grep -Fq "release artifact provenance version revision is not a lowercase hex commit token" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the malformed-version artifact for the wrong reason
expected: release artifact provenance version revision is not a lowercase hex commit token
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
stale_version_revision="deadbee"
if [[ "$version_output" =~ $version_revision_pattern ]]; then
  stale_version_output="$(
    printf '%s' "$version_output" | sed -E "s/@[^)]*\\)/@$stale_version_revision)/"
  )"
else
  stale_version_output="$version_output (stale@$stale_version_revision)"
fi
stale_version_slug="$(
  printf '%s' "${stale_version_output#modal }" |
    tr '[:upper:]' '[:lower:]' |
    sed -E 's/[^a-z0-9._-]+/-/g; s/^-+//; s/-+$//'
)"
stale_version_archive_name="modal-${stale_version_slug}-${os}-${arch}-${PROFILE}.tar.gz"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
sed "s/^version: .*$/version: $stale_version_output/" \
  "$STAGE_DIR/README.txt" >"$NEGATIVE_STAGE_DIR/README.txt"
sed "s/^version: .*$/version: $stale_version_output/" \
  "$STAGE_DIR/PROVENANCE.txt" >"$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
sed "s/artifact: $archive_name/artifact: $stale_version_archive_name/" \
  "$STAGE_DIR/EVIDENCE-BUNDLE.txt" >"$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$stale_version_archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$stale_version_archive_name" >"$stale_version_archive_name.sha256"
)
sed "s/$archive_name/$stale_version_archive_name/g" "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" \
  >"$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted provenance version metadata with a stale revision" >&2
  exit 1
}
if ! grep -Fq "release artifact provenance version revision does not match source revision" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the stale-version artifact for the wrong reason
expected: release artifact provenance version revision does not match source revision
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
bare_marker_version_output="$version_output stale @deadbee"
bare_marker_version_slug="$(
  printf '%s' "${bare_marker_version_output#modal }" |
    tr '[:upper:]' '[:lower:]' |
    sed -E 's/[^a-z0-9._-]+/-/g; s/^-+//; s/-+$//'
)"
bare_marker_archive_name="modal-${bare_marker_version_slug}-${os}-${arch}-${PROFILE}.tar.gz"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
sed "s/^version: .*$/version: $bare_marker_version_output/" \
  "$STAGE_DIR/README.txt" >"$NEGATIVE_STAGE_DIR/README.txt"
sed "s/^version: .*$/version: $bare_marker_version_output/" \
  "$STAGE_DIR/PROVENANCE.txt" >"$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
sed "s/artifact: $archive_name/artifact: $bare_marker_archive_name/" \
  "$STAGE_DIR/EVIDENCE-BUNDLE.txt" >"$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$bare_marker_archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$bare_marker_archive_name" >"$bare_marker_archive_name.sha256"
)
sed "s/$archive_name/$bare_marker_archive_name/g" "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" \
  >"$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted provenance version metadata with a bare revision marker" >&2
  exit 1
}
if ! grep -Fq "release artifact provenance version has an unsupported revision marker" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the bare-revision version artifact for the wrong reason
expected: release artifact provenance version has an unsupported revision marker
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cat "$STAGE_DIR/PROVENANCE.txt" >"$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cat >>"$NEGATIVE_STAGE_DIR/PROVENANCE.txt" <<EOF
modal release archive smoke provenance
EOF
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted ambiguous provenance marker metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact provenance has unexpected marker lines" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected ambiguous provenance marker metadata for the wrong reason
expected: release artifact provenance has unexpected marker lines
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cat >"$NEGATIVE_STAGE_DIR/bin/modal" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "--version" ]]; then
  echo "modal 0.0.0-stale"
  exit 0
fi
echo "stale modal test double" >&2
exit 1
EOF
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_SMOKE=1 \
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
  MODALITY_BIN="$STAGE_DIR/bin/modal" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted an unpacked modal binary with stale version output" >&2
  exit 1
}
if ! grep -Fq "release artifact unpacked modal version does not match provenance" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the stale-binary-version artifact for the wrong reason
expected: release artifact unpacked modal version does not match provenance
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cat >"$NEGATIVE_STAGE_DIR/bin/modal" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "--version" ]]; then
  printf '%s\\nstale extra modal version note\\n' "$version_output"
  exit 0
fi
echo "multi-line modal test double" >&2
exit 1
EOF
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_SMOKE=1 \
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
  MODALITY_BIN="$STAGE_DIR/bin/modal" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted an unpacked modal binary with multi-line version output" >&2
  exit 1
}
if ! grep -Fq "release artifact unpacked modal version is not a single line" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the multi-line unpacked modal version for the wrong reason
expected: release artifact unpacked modal version is not a single line
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cat >"$NEGATIVE_STAGE_DIR/bin/modal" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "--version" ]]; then
  printf '%s\\n\\n' "$version_output"
  exit 0
fi
echo "trailing-blank modal test double" >&2
exit 1
EOF
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_SMOKE=1 \
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
  MODALITY_BIN="$STAGE_DIR/bin/modal" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted an unpacked modal binary with a trailing blank version line" >&2
  exit 1
}
if ! grep -Fq "release artifact unpacked modal version is not a single line" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the trailing-blank unpacked modal version for the wrong reason
expected: release artifact unpacked modal version is not a single line
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
unsupported_arch="${arch}+stale"
unsupported_arch_archive_name="modal-${version_slug}-${os}-${unsupported_arch}-${PROFILE}.tar.gz"
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
cp "$STAGE_DIR/bin/modal" "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
sed "s/^arch: $arch\$/arch: $unsupported_arch/" \
  "$STAGE_DIR/PROVENANCE.txt" >"$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
sed "s/artifact: $archive_name/artifact: $unsupported_arch_archive_name/" \
  "$STAGE_DIR/EVIDENCE-BUNDLE.txt" >"$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar --no-recursion -czf "$NEGATIVE_ARTIFACT_DIR/$unsupported_arch_archive_name" \
    bin bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$unsupported_arch_archive_name" >"$unsupported_arch_archive_name.sha256"
)
sed "s/$archive_name/$unsupported_arch_archive_name/g" "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" \
  >"$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted unsupported provenance architecture metadata" >&2
  exit 1
}
if ! grep -Fq "release artifact provenance has unsupported arch value: $unsupported_arch" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the unsupported-arch artifact for the wrong reason
expected: release artifact provenance has unsupported arch value: $unsupported_arch
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_STAGE_DIR"
NEGATIVE_STAGE_DIR=""
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
sed -i '/^Expected downloaded directory entries:/,+3d' "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe without expected directory entries" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe is missing expected directory entries label" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the stale recipe for the wrong reason
expected: release artifact verification recipe is missing expected directory entries label
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
sed -i '/^  VERIFY-DOWNLOAD.txt$/a\  stale-extra-entry' "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe with extra downloaded directory entries" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe has unexpected downloaded directory entries" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the extra-entry recipe for the wrong reason
expected: release artifact verification recipe has unexpected downloaded directory entries
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
perl -0pi -e 's/(Expected downloaded directory entries:\n)  ([^\n]+)\n  ([^\n]+)\n  (VERIFY-DOWNLOAD\.txt)\n/${1}  $3\n  $2\n  $4\n/' \
  "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe with shuffled downloaded directory entries" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe has unexpected downloaded directory entries" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the shuffled-entry recipe for the wrong reason
expected: release artifact verification recipe has unexpected downloaded directory entries
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
sed -i '/^Expected source revision:$/,+2d' "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe without expected source revision" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe is missing expected source revision label" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the missing-source-revision recipe for the wrong reason
expected: release artifact verification recipe is missing expected source revision label
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
sed -i '/^Expected source revision:$/a\  stale-or-ambiguous' "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe with ambiguous expected revisions" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe has unexpected source revisions" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the ambiguous recipe revisions for the wrong reason
expected: release artifact verification recipe has unexpected source revisions
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
sed -i '/^Expected profile:$/,+2d' "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe without expected profile" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe is missing expected profile label" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the missing-profile recipe for the wrong reason
expected: release artifact verification recipe is missing expected profile label
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
sed -i '/^Expected profile:$/a\  stale-profile' "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe with ambiguous profiles" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe has unexpected profiles" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the ambiguous-profile recipe for the wrong reason
expected: release artifact verification recipe has unexpected profiles
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
sed -i '/^Expected feature set:$/,+2d' "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe without expected feature set" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe is missing expected feature set label" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the missing-feature-set recipe for the wrong reason
expected: release artifact verification recipe is missing expected feature set label
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
sed -i '/^Expected feature set:$/a\  stale-features' "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe with ambiguous feature sets" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe has unexpected feature sets" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the ambiguous-feature-set recipe for the wrong reason
expected: release artifact verification recipe has unexpected feature sets
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
sed -i '/^Expected help surface:$/,+2d' "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe without expected help surface" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe is missing expected help surface label" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the missing-help-surface recipe for the wrong reason
expected: release artifact verification recipe is missing expected help surface label
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
sed -i '/^Expected help surface:$/a\  stale-full' "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe with ambiguous help surfaces" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe has unexpected help surfaces" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the ambiguous help-surface recipe for the wrong reason
expected: release artifact verification recipe has unexpected help surfaces
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
perl -0pi -e 's/(Expected source revision:\n  [^\n]+\n\n)(Expected profile:\n  [^\n]+\n\nExpected feature set:\n  [^\n]+\n\n)(Expected help surface:\n  [^\n]+\n\n)/$3$2$1/' \
  "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe with shuffled sections" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe has unexpected section order" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the shuffled-section recipe for the wrong reason
expected: release artifact verification recipe has unexpected section order
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
sed -i '1i stale downloaded artifact note' "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe with a stale preamble before the title" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe title must be the first line" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the preamble recipe for the wrong reason
expected: release artifact verification recipe title must be the first line
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
cat >>"$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt" <<EOF
modal release archive download verification
EOF
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe with duplicate title lines" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe has unexpected title lines" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the duplicate-title recipe for the wrong reason
expected: release artifact verification recipe has unexpected title lines
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
perl -0pi -e 's/\nVerify before unpacking or trusting the binary:\n  sha256sum -c [^\n]+\n  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV=[^\n]+check-modal-release-artifact-download\.sh \/path\/to\/downloaded-artifact-dir\n//' \
  "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe without a verification section" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe has unexpected section order" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the missing-verification-section recipe for the wrong reason
expected: release artifact verification recipe has unexpected section order
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
grep -Fv "MODAL_ONBOARDING_ARTIFACT_SMOKE=1" "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" \
  >"$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe without smoke replay environment" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe has unexpected smoke replay environment lines" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the missing-smoke recipe for the wrong reason
expected: release artifact verification recipe has unexpected smoke replay environment lines
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
perl -0pi -e 's/(Add MODAL_ONBOARDING_ARTIFACT_SMOKE=1 and MODALITY_BIN=\/path\/to\/modality built\n)/$1stale smoke replay note\n/' \
  "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe with a split smoke replay trailer" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe has unexpected smoke replay trailer" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the split-smoke recipe for the wrong reason
expected: release artifact verification recipe has unexpected smoke replay trailer
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
cat >>"$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt" <<EOF
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV=stale-or-ambiguous tests/cli/check-modal-release-artifact-download.sh /path/to/downloaded-artifact-dir
  $expected_verify_command
EOF
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe with ambiguous verifier commands" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe has unexpected verifier command lines" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the ambiguous verifier-command recipe for the wrong reason
expected: release artifact verification recipe has unexpected verifier command lines
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
perl -0pi -e 's/(  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV=[^\n]+check-modal-release-artifact-download\.sh \/path\/to\/downloaded-artifact-dir\n)/$1  echo stale-extra-verification-step\n/' \
  "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe with an extra verification command" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe has unexpected verification commands" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the extra-command recipe for the wrong reason
expected: release artifact verification recipe has unexpected verification commands
actual:
$negative_output
EOF
  exit 1
fi
rm -rf "$NEGATIVE_ARTIFACT_DIR"
NEGATIVE_ARTIFACT_DIR="$(mktemp -d)"
cp "$ARCHIVE_DIR/$archive_name" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/$archive_name.sha256" "$NEGATIVE_ARTIFACT_DIR/"
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/"
perl -0pi -e 's/(\nExpected help surface:\n)/\nStale note: this recipe was hand-merged from another run.\n$1/' \
  "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
    "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$NEGATIVE_ARTIFACT_DIR" 2>&1
)" && {
  echo "release artifact verifier accepted a recipe with stale inter-section prose" >&2
  exit 1
}
if ! grep -Fq "release artifact verification recipe is not the canonical emitted recipe" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the stale inter-section recipe for the wrong reason
expected: release artifact verification recipe is not the canonical emitted recipe
actual:
$negative_output
EOF
  exit 1
fi
negative_output="$(
  MODAL_ONBOARDING_PROFILE="smoke" \
  MODAL_ONBOARDING_ARCHIVE_EXPECT_REV="" \
    "$ROOT_DIR/tests/cli/check-modal-release-archive-readiness.sh" 2>&1
)" && {
  echo "release archive readiness accepted an unsupported producer build profile" >&2
  exit 1
}
if ! grep -Fq "unsupported MODAL_ONBOARDING_PROFILE: smoke" <<<"$negative_output"; then
  cat >&2 <<EOF
release archive readiness rejected unsupported producer build profile for the wrong reason
expected: unsupported MODAL_ONBOARDING_PROFILE: smoke
actual:
$negative_output
EOF
  exit 1
fi
negative_output="$(
  MODAL_ONBOARDING_ARCHIVE_REV="unknown" \
  MODAL_ONBOARDING_ARCHIVE_EXPECT_REV="" \
    "$ROOT_DIR/tests/cli/check-modal-release-archive-readiness.sh" 2>&1
)" && {
  echo "release archive readiness accepted an unsupported producer source revision" >&2
  exit 1
}
if ! grep -Fq "release archive source revision is not an archive-safe commit token" <<<"$negative_output"; then
  cat >&2 <<EOF
release archive readiness rejected unsupported producer source revision for the wrong reason
expected: release archive source revision is not an archive-safe commit token
actual:
$negative_output
EOF
  exit 1
fi
negative_output="$(
  MODAL_ONBOARDING_ARCHIVE_ARCH="${arch}+stale" \
  MODAL_ONBOARDING_ARCHIVE_EXPECT_REV="" \
    "$ROOT_DIR/tests/cli/check-modal-release-archive-readiness.sh" 2>&1
)" && {
  echo "release archive readiness accepted unsafe producer platform metadata" >&2
  exit 1
}
if ! grep -Fq "release archive platform metadata is not archive-safe" <<<"$negative_output"; then
  cat >&2 <<EOF
release archive readiness rejected unsafe producer platform metadata for the wrong reason
expected: release archive platform metadata is not archive-safe
actual:
$negative_output
EOF
  exit 1
fi
negative_output="$(
  MODAL_HELP_SURFACE="experimental" \
  MODAL_ONBOARDING_ARCHIVE_EXPECT_REV="" \
    "$ROOT_DIR/tests/cli/check-modal-release-archive-readiness.sh" 2>&1
)" && {
  echo "release archive readiness accepted an unsupported producer help surface" >&2
  exit 1
}
if ! grep -Fq "release archive help surface is not supported" <<<"$negative_output"; then
  cat >&2 <<EOF
release archive readiness rejected unsupported producer help surface for the wrong reason
expected: release archive help surface is not supported
actual:
$negative_output
EOF
  exit 1
fi
negative_output="$(
  MODAL_ONBOARDING_FEATURES="experimental-features" \
  MODAL_ONBOARDING_ARCHIVE_EXPECT_REV="" \
    "$ROOT_DIR/tests/cli/check-modal-release-archive-readiness.sh" 2>&1
)" && {
  echo "release archive readiness accepted an unsupported producer feature set" >&2
  exit 1
}
if ! grep -Fq "release archive feature set is not supported" <<<"$negative_output"; then
  cat >&2 <<EOF
release archive readiness rejected unsupported producer feature set for the wrong reason
expected: release archive feature set is not supported
actual:
$negative_output
EOF
  exit 1
fi
negative_output="$(
  MODAL_HELP_SURFACE="full" \
  MODAL_ONBOARDING_FEATURES="contract-onboarding" \
  MODAL_ONBOARDING_ARCHIVE_EXPECT_REV="" \
    "$ROOT_DIR/tests/cli/check-modal-release-archive-readiness.sh" 2>&1
)" && {
  echo "release archive readiness accepted mismatched producer feature/help-surface metadata" >&2
  exit 1
}
if ! grep -Fq "release archive help surface does not match feature set" <<<"$negative_output"; then
  cat >&2 <<EOF
release archive readiness rejected mismatched producer feature/help metadata for the wrong reason
expected: release archive help surface does not match feature set
actual:
$negative_output
EOF
  exit 1
fi

archive_listing="$(tar -tzf "$ARCHIVE_PATH")"
expected_archive_listing="$(
  printf '%s\n' \
    "bin/" \
    "bin/modal" \
    "README.txt" \
    "PROVENANCE.txt" \
    "EVIDENCE-BUNDLE.txt" \
    "SHA256SUMS"
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
if ! grep -Fxq "bin/" <<<"$archive_listing"; then
  echo "release archive is missing bin/ directory" >&2
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
if [[ ! -d "$UNPACK_DIR/bin" || -L "$UNPACK_DIR/bin" ]]; then
  echo "release archive unpacked bin entry must be a regular non-symlink directory" >&2
  exit 1
fi
for required_unpacked_path in \
  "$UNPACK_DIR/bin/modal" \
  "$UNPACK_DIR/README.txt" \
  "$UNPACK_DIR/PROVENANCE.txt" \
  "$UNPACK_DIR/EVIDENCE-BUNDLE.txt" \
  "$UNPACK_DIR/SHA256SUMS"
do
  if [[ ! -f "$required_unpacked_path" || -L "$required_unpacked_path" ]]; then
    relative_path="${required_unpacked_path#"$UNPACK_DIR"/}"
    printf 'release archive unpacked entry must be a regular non-symlink file: %s\n' \
      "$relative_path" >&2
    exit 1
  fi
done
check_unpacked_mode() {
  local relative_path="$1"
  local expected_mode="$2"
  local actual_mode
  actual_mode="$(stat -c '%a' "$UNPACK_DIR/$relative_path")"
  if [[ "$actual_mode" != "$expected_mode" ]]; then
    cat >&2 <<EOF
release archive unpacked entry has unexpected mode: $relative_path
expected: $expected_mode
actual:   $actual_mode
EOF
    exit 1
  fi
}
check_unpacked_mode "bin" "755"
check_unpacked_mode "bin/modal" "755"
check_unpacked_mode "README.txt" "644"
check_unpacked_mode "PROVENANCE.txt" "644"
check_unpacked_mode "EVIDENCE-BUNDLE.txt" "644"
check_unpacked_mode "SHA256SUMS" "644"
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
if ! grep -Fq "version: $version_output" "$UNPACK_DIR/EVIDENCE-BUNDLE.txt"; then
  echo "release archive evidence manifest is missing version: $version_output" >&2
  exit 1
fi
if ! grep -Fq "source revision: $source_revision" "$UNPACK_DIR/EVIDENCE-BUNDLE.txt"; then
  echo "release archive evidence manifest is missing source revision: $source_revision" >&2
  exit 1
fi
if ! grep -Fq "profile: $PROFILE" "$UNPACK_DIR/EVIDENCE-BUNDLE.txt"; then
  echo "release archive evidence manifest is missing profile: $PROFILE" >&2
  exit 1
fi
if ! grep -Fq "features: $FEATURES" "$UNPACK_DIR/EVIDENCE-BUNDLE.txt"; then
  echo "release archive evidence manifest is missing features: $FEATURES" >&2
  exit 1
fi
if ! grep -Fq "post-unpack checks: version, help surface, same-revision language CLI, first-contract smoke when artifact smoke is enabled" "$UNPACK_DIR/EVIDENCE-BUNDLE.txt"; then
  echo "release archive evidence manifest is missing post-unpack checks" >&2
  exit 1
fi
UNPACKED_MODAL="$UNPACK_DIR/bin/modal"
if [[ ! -x "$UNPACKED_MODAL" ]]; then
  echo "unpacked modal is not executable at $UNPACKED_MODAL" >&2
  exit 1
fi

capture_command_output_lines "$UNPACKED_MODAL" --version
if [[ "${#captured_output_lines[@]}" -ne 1 ]]; then
  unpacked_version="$(captured_output_as_text)"
  cat >&2 <<EOF
release archive unpacked modal version is not a single line
expected: $version_output
actual:
$unpacked_version
EOF
  exit 1
fi
unpacked_version="${captured_output_lines[0]}"
if [[ "$unpacked_version" != "$version_output" ]]; then
  echo "unpacked modal version changed: $unpacked_version (expected $version_output)" >&2
  exit 1
fi

MODAL_BIN="$UNPACKED_MODAL" MODAL_HELP_SURFACE="$HELP_SURFACE" \
  "$ROOT_DIR/tests/cli/check-modal-help-surface.sh"

if [[ -x "${MODALITY_BIN:-}" ]]; then
  if [[ ! -f "$MODALITY_BIN" || -L "$MODALITY_BIN" ]]; then
    cat >&2 <<EOF
release archive smoke replay needs a regular non-symlink MODALITY_BIN
actual: $MODALITY_BIN

Set MODALITY_BIN=/path/to/modality built from the same source revision to run
the first-contract smoke against the unpacked modal binary.
EOF
    exit 2
  fi
  capture_command_output_lines "$MODALITY_BIN" --version
  if [[ "${#captured_output_lines[@]}" -ne 1 ]]; then
    modality_version="$(captured_output_as_text)"
    cat >&2 <<EOF
release archive smoke modality version is not a single line
expected revision: $source_revision
actual version:
$modality_version
EOF
    exit 1
  fi
  modality_version="${captured_output_lines[0]}"
  case "$modality_version" in
    modality\ *)
      ;;
    *)
      cat >&2 <<EOF
release archive smoke modality binary reported an unexpected version prefix
expected prefix: modality
actual version:  $modality_version

Set MODALITY_BIN=/path/to/modality built from the same source revision to run
the first-contract smoke against the unpacked modal binary.
EOF
      exit 1
      ;;
  esac
  modality_revision_pattern='\([^)]*@([^)]+)\)'
  modality_revision_marker_count="$(
    grep -Eo '\([^)]*@[^)]+\)' <<<"$modality_version" | wc -l || true
  )"
  modality_revision_at_count="$(
    grep -o '@' <<<"$modality_version" | wc -l || true
  )"
  if [[ "$modality_revision_marker_count" -gt 1 ]]; then
    cat >&2 <<EOF
release archive smoke modality version has multiple revision markers
expected revision: $source_revision
actual version:    $modality_version
EOF
    exit 1
  fi
  if [[ "$modality_revision_at_count" -ne "$modality_revision_marker_count" ]]; then
    cat >&2 <<EOF
release archive smoke modality version has an unsupported revision marker
expected revision: $source_revision
actual version:    $modality_version

Use at most one parenthesized source revision marker ending in @<commit>.
EOF
    exit 1
  fi
  if [[ ! "$modality_version" =~ $modality_revision_pattern ]]; then
    cat >&2 <<EOF
release archive smoke modality version does not include a source revision
expected revision: $source_revision
actual version:    $modality_version
EOF
    exit 1
  fi
  modality_revision="${BASH_REMATCH[1]}"
  if [[ ! "$modality_revision" =~ ^[0-9a-f]{7,40}$ ]]; then
    cat >&2 <<EOF
release archive smoke modality version revision is not a lowercase hex commit token
expected revision: $source_revision
actual version:    $modality_version

Build modality from a Git checkout that reports a full commit hash or an
unambiguous Git-style short hash of at least seven hexadecimal characters.
EOF
    exit 1
  fi
  if ! revisions_match "$source_revision" "$modality_revision"; then
    cat >&2 <<EOF
release archive smoke modality version does not match source revision
expected revision: $source_revision
actual version:    $modality_version
EOF
    exit 1
  fi
  MODAL_BIN="$UNPACKED_MODAL" MODALITY_BIN="$MODALITY_BIN" \
    "$ROOT_DIR/tests/cli/run-first-contract-cli-smoke.sh"
else
  cat <<EOF
first-contract release-archive smoke skipped: MODALITY_BIN not supplied

Pass a built language CLI to verify the unpacked modal binary against the full
first-contract path:
  MODALITY_BIN=/path/to/modality-built-from-$source_revision $0
EOF
fi

echo "modal release archive readiness check passed: $archive_name"
