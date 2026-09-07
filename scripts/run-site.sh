#!/usr/bin/env bash

# Start the local Docusaurus docs site (sites/www.modality.org).
# Syncs repo-root docs/ into the site, matching CI, then runs `npm start`.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SITE_DIR="$PROJECT_ROOT/sites/www.modality.org"
DOCS_SRC="$PROJECT_ROOT/docs"

BLUE='\033[0;34m'
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

show_help() {
    cat << EOF
Start the local Modality docs site

USAGE:
    $0 [OPTIONS] [-- DOCUSAURUS_ARGS...]

Copies docs/ into sites/www.modality.org/docs (same as CI), installs npm
dependencies if needed, then starts Docusaurus. Extra arguments are passed
to \`docusaurus start\`.

OPTIONS:
    --no-sync              Do not copy repo-root docs/ into the site
    -h, --help             Show this help message

EXAMPLES:
    $0
    $0 --port 3001
    $0 --no-sync -- --no-open

The site is usually at http://localhost:3000
First contract: http://localhost:3000/docs/getting-started/first-contract

EOF
}

SYNC_DOCS=true
PASSTHROUGH=()

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        --no-sync)
            SYNC_DOCS=false
            shift
            ;;
        --)
            shift
            PASSTHROUGH+=("$@")
            break
            ;;
        *)
            PASSTHROUGH+=("$1")
            shift
            ;;
    esac
done

if [[ ! -d "$SITE_DIR" ]]; then
    log_error "Docs site not found at $SITE_DIR"
    exit 1
fi

if ! command -v node >/dev/null 2>&1; then
    log_error "Node.js is required (site engines: >=20)"
    exit 1
fi

NODE_MAJOR="$(node -p "process.versions.node.split('.')[0]")"
if [[ "$NODE_MAJOR" -lt 20 ]]; then
    log_error "Node.js >= 20 is required (found $(node -v))"
    exit 1
fi

if ! command -v npm >/dev/null 2>&1; then
    log_error "npm is required"
    exit 1
fi

cd "$SITE_DIR"

if [[ "$SYNC_DOCS" == true ]]; then
    if [[ ! -d "$DOCS_SRC" ]]; then
        log_error "Docs source not found at $DOCS_SRC"
        exit 1
    fi
    log_info "Syncing $DOCS_SRC -> $SITE_DIR/docs"
    rm -rf "$SITE_DIR/docs"
    cp -R "$DOCS_SRC" "$SITE_DIR/docs"
fi

if [[ ! -x "$SITE_DIR/node_modules/.bin/docusaurus" ]]; then
    log_info "Installing docs site dependencies..."
    if [[ -f package-lock.json ]]; then
        npm ci
    else
        npm install
    fi
fi

log_success "Starting docs site (http://localhost:3000)"
if [[ ${#PASSTHROUGH[@]} -gt 0 ]]; then
    exec npm start -- "${PASSTHROUGH[@]}"
else
    exec npm start
fi
