#!/usr/bin/env bash
# Modality installation script.
#
# Installs `modal` for this platform, checks it against the release's
# SHA256SUMS, then has it check that its release is one the channel's release
# contract accepts. This script is served from the same place as the binary,
# so it cannot defend against that place itself: to check independently,
# build `modal` from source and run
#   modal release verify --log https://get.modality.org/testnet/release-contract/log.json \
#     --channel testnet --file binaries/<platform>/modal=<path to modal>
set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { printf "${BLUE}[INFO]${NC} %s\n" "$1"; }
log_success() { printf "${GREEN}[SUCCESS]${NC} %s\n" "$1"; }
log_error() { printf "${RED}[ERROR]${NC} %s\n" "$1"; }

detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"
    case "$os" in
        Linux*)
            case "$arch" in
                x86_64) echo "linux-x86_64" ;;
                *) log_error "Unsupported architecture: $arch"; exit 1 ;;
            esac
            ;;
        Darwin*)
            case "$arch" in
                arm64) echo "darwin-aarch64" ;;
                *) log_error "Unsupported architecture: $arch"; exit 1 ;;
            esac
            ;;
        *)
            log_error "Unsupported operating system: $os"
            exit 1
            ;;
    esac
}

fetch() {
    if command -v curl > /dev/null 2>&1; then
        curl -fsSL "$1" -o "$2"
    elif command -v wget > /dev/null 2>&1; then
        wget -q "$1" -O "$2"
    else
        log_error "Neither curl nor wget found"
        exit 1
    fi
}

sha256_of() {
    if command -v sha256sum > /dev/null 2>&1; then
        sha256sum "$1" | cut -d' ' -f1
    else
        shasum -a 256 "$1" | cut -d' ' -f1
    fi
}

PLATFORM=$(detect_platform)
CHANNEL="${MODALITY_CHANNEL:-testnet}"
BASE_URL="${MODALITY_INSTALL_URL:-https://get.modality.org/$CHANNEL/latest}"
INSTALL_DIR="${MODALITY_INSTALL_DIR:-$HOME/.modality/bin}"
BINARY_NAME="modal"

log_info "Detected platform: $PLATFORM"
log_info "Installing to: $INSTALL_DIR"
mkdir -p "$INSTALL_DIR"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fetch "$BASE_URL/manifest.json" "$TMP/manifest.json"
VERSION=$(sed -n 's/.*"version": *"\([^"]*\)".*/\1/p' "$TMP/manifest.json" | head -1)
[ -n "$VERSION" ] || { log_error "No version in $BASE_URL/manifest.json"; exit 1; }
# The versioned directory: `latest` can move between reads.
RELEASE_URL="${BASE_URL%/latest}/$VERSION"
log_info "Release: $VERSION"

fetch "$RELEASE_URL/binaries/$PLATFORM/$BINARY_NAME" "$TMP/$BINARY_NAME"
fetch "$RELEASE_URL/SHA256SUMS" "$TMP/SHA256SUMS"
EXPECTED=$(grep " binaries/$PLATFORM/$BINARY_NAME\$" "$TMP/SHA256SUMS" | cut -d' ' -f1)
ACTUAL=$(sha256_of "$TMP/$BINARY_NAME")
if [ -z "$EXPECTED" ] || [ "$EXPECTED" != "$ACTUAL" ]; then
    log_error "The binary's sha256 ($ACTUAL) is not the release's (${EXPECTED:-none})."
    exit 1
fi
chmod +x "$TMP/$BINARY_NAME"

if "$TMP/$BINARY_NAME" release verify \
    --log "${BASE_URL%/latest}/release-contract/log.json" --channel "$CHANNEL" \
    --version "$VERSION" --file "binaries/$PLATFORM/$BINARY_NAME=$TMP/$BINARY_NAME" > "$TMP/verify.out" 2>&1; then
    log_info "$(tail -1 "$TMP/verify.out")"
else
    log_error "The release contract does not accept this binary:"
    cat "$TMP/verify.out" >&2
    exit 1
fi

mv "$TMP/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
log_success "Modality installed successfully!"
log_info "Binary location: $INSTALL_DIR/$BINARY_NAME"

case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        printf "\n"
        log_info "To use modal, add it to your PATH:"
        log_info "  export PATH=\"\$PATH:$INSTALL_DIR\""
        ;;
esac
