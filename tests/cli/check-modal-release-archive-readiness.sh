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

Verify before unpacking or trusting the binary:
  sha256sum -c $archive_name.sha256
  MODAL_ONBOARDING_ARTIFACT_EXPECT_REV=$source_revision tests/cli/check-modal-release-artifact-download.sh /path/to/downloaded-artifact-dir

Add MODAL_ONBOARDING_ARTIFACT_SMOKE=1 and MODALITY_BIN=/path/to/modality to
run the help-surface and first-contract smokes against the unpacked modal binary.
EOF
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
if ! grep -Fq "release artifact detached checksum sidecar has unexpected entries" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the duplicated detached checksum sidecar for the wrong reason
expected: release artifact detached checksum sidecar has unexpected entries
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
if ! grep -Fq "release artifact README is missing help surface: $HELP_SURFACE" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected stale README metadata for the wrong reason
expected: release artifact README is missing help surface: $HELP_SURFACE
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
if ! grep -Fq "release artifact evidence manifest is missing checksums: SHA256SUMS" <<<"$negative_output"; then
  cat >&2 <<EOF
release artifact verifier rejected the stale evidence manifest for the wrong reason
expected: release artifact evidence manifest is missing checksums: SHA256SUMS
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
