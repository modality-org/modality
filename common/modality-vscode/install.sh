#!/bin/bash
# Install the Modality extension into VS Code, Cursor and VSCodium.
#
# Builds a .vsix and installs it with each editor's CLI. The .vsix carries
# the extension's runtime dependencies; a plain file copy does not.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Installing dependencies..."
npm install

echo "Checking the grammar..."
npm run test:grammar

echo "Packaging..."
npx vsce package --skip-license -o modality-vscode.vsix

INSTALLED=0
for cli in code cursor codium; do
    if command -v "$cli" >/dev/null 2>&1; then
        echo "Installing with $cli..."
        "$cli" --install-extension modality-vscode.vsix --force
        INSTALLED=1
    fi
done

if [ $INSTALLED -eq 0 ]; then
    echo "No editor CLI found (code, cursor, codium)."
    echo "Install $SCRIPT_DIR/modality-vscode.vsix from the Extensions view: ... > Install from VSIX."
    exit 1
fi

echo ""
echo "Installed. Run 'Developer: Reload Window' in your editor."
echo ""
echo "Optional: the modality-lsp language server (diagnostics, hover, outline):"
echo "  cd rust && cargo install --path modality-lsp"
