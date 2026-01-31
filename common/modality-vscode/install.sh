#!/bin/bash
# Install Modality VSCode extension via symbolic link
# Works for both VSCode and Cursor

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXT_NAME="modality-vscode"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

install_extension() {
    local editor_name=$1
    local ext_dir=$2
    
    if [ -d "$ext_dir" ]; then
        local target="$ext_dir/$EXT_NAME"
        
        # Remove existing if present
        if [ -L "$target" ] || [ -d "$target" ]; then
            echo -e "${YELLOW}Removing existing $editor_name extension...${NC}"
            rm -rf "$target"
        fi
        
        # Create symlink
        ln -s "$SCRIPT_DIR" "$target"
        echo -e "${GREEN}✓ Installed for $editor_name${NC}"
        echo "  $target -> $SCRIPT_DIR"
        return 0
    else
        return 1
    fi
}

echo "Installing Modality extension..."
echo

INSTALLED=0

# VSCode
if [ -d "$HOME/.vscode/extensions" ]; then
    install_extension "VSCode" "$HOME/.vscode/extensions" && ((INSTALLED++))
elif [ -d "$HOME/.vscode-server/extensions" ]; then
    install_extension "VSCode Server" "$HOME/.vscode-server/extensions" && ((INSTALLED++))
fi

# Cursor
if [ -d "$HOME/.cursor/extensions" ]; then
    install_extension "Cursor" "$HOME/.cursor/extensions" && ((INSTALLED++))
fi

# VSCodium
if [ -d "$HOME/.vscode-oss/extensions" ]; then
    install_extension "VSCodium" "$HOME/.vscode-oss/extensions" && ((INSTALLED++))
fi

# macOS paths
if [[ "$OSTYPE" == "darwin"* ]]; then
    if [ -d "$HOME/Library/Application Support/Code/User/extensions" ]; then
        install_extension "VSCode (macOS)" "$HOME/Library/Application Support/Code/User/extensions" && ((INSTALLED++))
    fi
    if [ -d "$HOME/Library/Application Support/Cursor/User/extensions" ]; then
        install_extension "Cursor (macOS)" "$HOME/Library/Application Support/Cursor/User/extensions" && ((INSTALLED++))
    fi
fi

echo
if [ $INSTALLED -gt 0 ]; then
    echo -e "${GREEN}Done! Restart your editor to activate.${NC}"
else
    echo -e "${RED}No supported editors found.${NC}"
    echo "Looked for:"
    echo "  - ~/.vscode/extensions"
    echo "  - ~/.cursor/extensions"
    echo "  - ~/.vscode-oss/extensions"
    exit 1
fi
