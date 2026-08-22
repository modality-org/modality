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
if [[ ! "$source_revision" =~ ^[0-9a-f]{7,40}$ ]]; then
  cat >&2 <<EOF
release archive source revision is not an archive-safe commit token
actual: $source_revision

Build modal from a Git checkout, or set MODAL_ONBOARDING_ARCHIVE_REV to the
lowercase source commit used for this archive.
EOF
  exit 1
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
chmod 0644 "$STAGE_DIR/README.txt" "$STAGE_DIR/PROVENANCE.txt" "$STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
)

ARCHIVE_PATH="$ARCHIVE_DIR/$archive_name"
tar -C "$STAGE_DIR" -czf "$ARCHIVE_PATH" \
  bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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

Expected help surface:
  $HELP_SURFACE

Verify before unpacking or trusting the binary:
  sha256sum -c $archive_name.sha256
  $expected_verify_command

Add MODAL_ONBOARDING_ARTIFACT_SMOKE=1 and MODALITY_BIN=/path/to/modality to
run the help-surface and first-contract smokes against the unpacked modal binary.
EOF
chmod 0644 "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt"
MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
  "$ROOT_DIR/tests/cli/check-modal-release-artifact-download.sh" "$ARCHIVE_DIR" >/dev/null
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
NEGATIVE_STAGE_DIR="$(mktemp -d)"
mkdir -p "$NEGATIVE_STAGE_DIR/bin"
ln -s /bin/sh "$NEGATIVE_STAGE_DIR/bin/modal"
cp "$STAGE_DIR/README.txt" "$NEGATIVE_STAGE_DIR/README.txt"
cp "$STAGE_DIR/PROVENANCE.txt" "$NEGATIVE_STAGE_DIR/PROVENANCE.txt"
cp "$STAGE_DIR/EVIDENCE-BUNDLE.txt" "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
chmod 0755 "$NEGATIVE_STAGE_DIR/bin/modal"
chmod 0644 \
  "$NEGATIVE_STAGE_DIR/README.txt" \
  "$NEGATIVE_STAGE_DIR/PROVENANCE.txt" \
  "$NEGATIVE_STAGE_DIR/EVIDENCE-BUNDLE.txt"
(
  cd "$NEGATIVE_STAGE_DIR"
  sha256sum bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt >SHA256SUMS
  chmod 0644 SHA256SUMS
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    README.txt bin/modal PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$wrong_archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$unsupported_profile_archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
sed "s/^features: ${MODAL_ONBOARDING_FEATURES:-contract-onboarding}\$/features: $unsupported_features/" \
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
stale_version_revision="staleabc"
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$stale_version_archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
)
(
  cd "$NEGATIVE_ARTIFACT_DIR"
  sha256sum "$archive_name" >"$archive_name.sha256"
)
cp "$ARCHIVE_DIR/VERIFY-DOWNLOAD.txt" "$NEGATIVE_ARTIFACT_DIR/VERIFY-DOWNLOAD.txt"
negative_output="$(
  MODAL_ONBOARDING_ARTIFACT_SMOKE=1 \
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV="${MODAL_ONBOARDING_ARCHIVE_EXPECT_REV:-}" \
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
  tar -czf "$NEGATIVE_ARTIFACT_DIR/$unsupported_arch_archive_name" \
    bin/modal README.txt PROVENANCE.txt EVIDENCE-BUNDLE.txt SHA256SUMS
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
perl -0pi -e 's/(Expected source revision:\n  [^\n]+\n\n)(Expected help surface:\n  [^\n]+\n\n)/$2$1/' \
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
perl -0pi -e 's/(Add MODAL_ONBOARDING_ARTIFACT_SMOKE=1 and MODALITY_BIN=\/path\/to\/modality to\n)/$1stale smoke replay note\n/' \
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
