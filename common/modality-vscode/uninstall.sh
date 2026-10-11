#!/bin/bash
# Uninstall the Modality extension from VS Code, Cursor and VSCodium.

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

REMOVED=0

for cli in code cursor codium; do
    if command -v "$cli" >/dev/null 2>&1 && "$cli" --list-extensions | grep -qx 'modality-org.modality-vscode'; then
        "$cli" --uninstall-extension modality-org.modality-vscode && ((REMOVED++))
    fi
done

# Copies and symlinks left by the old install script.
for dir in \
    "$HOME/.vscode/extensions" \
    "$HOME/.vscode-server/extensions" \
    "$HOME/.cursor/extensions" \
    "$HOME/.vscode-oss/extensions"
do
    target="$dir/modality-vscode"
    if [ -L "$target" ]; then
        rm "$target"
        echo -e "${GREEN}✓ Removed link $target${NC}"
        ((REMOVED++))
    elif [ -d "$target" ]; then
        rm -rf "$target"
        echo -e "${GREEN}✓ Removed $target${NC}"
        ((REMOVED++))
    fi
done

echo
if [ $REMOVED -gt 0 ]; then
    echo -e "${GREEN}Done! Reload your editor.${NC}"
else
    echo -e "${YELLOW}No installations found.${NC}"
fi
