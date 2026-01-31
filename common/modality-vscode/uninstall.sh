#!/bin/bash
# Uninstall Modality VSCode extension

EXT_NAME="modality-vscode"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

uninstall_extension() {
    local editor_name=$1
    local ext_dir=$2
    local target="$ext_dir/$EXT_NAME"
    
    if [ -L "$target" ] || [ -d "$target" ]; then
        rm -rf "$target"
        echo -e "${GREEN}✓ Removed from $editor_name${NC}"
        return 0
    fi
    return 1
}

echo "Uninstalling Modality extension..."
echo

REMOVED=0

# Check all possible locations
for dir in \
    "$HOME/.vscode/extensions" \
    "$HOME/.vscode-server/extensions" \
    "$HOME/.cursor/extensions" \
    "$HOME/.vscode-oss/extensions" \
    "$HOME/Library/Application Support/Code/User/extensions" \
    "$HOME/Library/Application Support/Cursor/User/extensions"
do
    if [ -d "$dir" ]; then
        uninstall_extension "${dir##*/}" "$dir" && ((REMOVED++))
    fi
done

echo
if [ $REMOVED -gt 0 ]; then
    echo -e "${GREEN}Done! Restart your editor.${NC}"
else
    echo -e "${YELLOW}No installations found.${NC}"
fi
