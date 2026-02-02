#!/bin/bash
# Install Modality VSCode Extension

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Find VS Code extensions directory
if [[ "$OSTYPE" == "darwin"* ]]; then
    VSCODE_DIR="$HOME/.vscode/extensions"
    CURSOR_DIR="$HOME/.cursor/extensions"
    VSCODIUM_DIR="$HOME/.vscode-oss/extensions"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    VSCODE_DIR="$HOME/.vscode/extensions"
    CURSOR_DIR="$HOME/.cursor/extensions"
    VSCODIUM_DIR="$HOME/.vscode-oss/extensions"
else
    VSCODE_DIR="$APPDATA/Code/User/extensions"
    CURSOR_DIR="$APPDATA/Cursor/User/extensions"
    VSCODIUM_DIR="$APPDATA/VSCodium/User/extensions"
fi

EXT_NAME="modality-vscode"

# Install npm dependencies
echo "Installing dependencies..."
npm install

# Compile TypeScript
echo "Compiling extension..."
npm run compile

# Install to all found editors
install_to() {
    local dir="$1"
    local name="$2"
    
    if [ -d "$dir" ] || [ -d "$(dirname "$dir")" ]; then
        echo "Installing to $name..."
        mkdir -p "$dir/$EXT_NAME"
        cp -r package.json language-configuration.json syntaxes out "$dir/$EXT_NAME/"
        if [ -d "examples" ]; then
            cp -r examples "$dir/$EXT_NAME/"
        fi
        echo "✓ Installed to $name"
    fi
}

install_to "$VSCODE_DIR" "VS Code"
install_to "$CURSOR_DIR" "Cursor"
install_to "$VSCODIUM_DIR" "VSCodium"

echo ""
echo "Installation complete!"
echo ""
echo "Make sure modality-lsp is installed:"
echo "  cd rust && cargo install --path modality-lsp"
echo ""
echo "Restart your editor to activate the extension."
