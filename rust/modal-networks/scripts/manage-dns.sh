#!/bin/bash
# Script for managing Modality network DNS records
# Usage: ./update-dns.sh [command] [options]

set -e

cd "$(dirname "$0")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Build the binary if needed
if [ ! -f "../../target/debug/modal-networks" ] || [ "src/main.rs" -nt "../../target/debug/modal-networks" ]; then
    echo -e "${YELLOW}Building modal-networks binary...${NC}"
    cargo build
fi

BINARY="../../target/debug/modal-networks"

# Default command
COMMAND="${1:-help}"

case "$COMMAND" in
    list)
        echo -e "${GREEN}Listing all networks:${NC}"
        $BINARY list
        ;;
    show)
        if [ -z "$2" ]; then
            echo -e "${RED}Error: Please specify a network name${NC}"
            echo "Usage: $0 show <network>"
            exit 1
        fi
        $BINARY show "$2"
        ;;
    update)
        NETWORK="${2:-}"
        DRY_RUN="${3:-}"
        
        if [ "$DRY_RUN" = "--dry-run" ] || [ "$DRY_RUN" = "-d" ]; then
            echo -e "${YELLOW}Running in dry-run mode (no changes will be made)${NC}"
            if [ -n "$NETWORK" ]; then
                $BINARY update-dns --network "$NETWORK" --dry-run
            else
                $BINARY update-dns --dry-run
            fi
        else
            echo -e "${YELLOW}Updating DNS records...${NC}"
            if [ -n "$NETWORK" ]; then
                $BINARY update-dns --network "$NETWORK"
            else
                $BINARY update-dns
            fi
            echo -e "${GREEN}DNS update complete!${NC}"
        fi
        ;;
    verify)
        NETWORK="${2:-testnet}"
        echo -e "${GREEN}Verifying DNS records for $NETWORK:${NC}"
        dig +short txt "_dnsaddr.$NETWORK.modality.network"
        ;;
    help)
        echo "Modality Network DNS Management Script"
        echo ""
        echo "Usage: $0 <command> [options]"
        echo ""
        echo "Commands:"
        echo "  list                    - List all available networks"
        echo "  show <network>          - Show details for a specific network"
        echo "  update [network] [--dry-run]"
        echo "                          - Update DNS records (all networks or specific)"
        echo "                            Use --dry-run to preview changes"
        echo "  verify [network]        - Verify DNS records using dig (default: testnet)"
        echo "  help                    - Show this help message"
        echo ""
        echo "Examples:"
        echo "  $0 list"
        echo "  $0 show testnet"
        echo "  $0 update --dry-run"
        echo "  $0 update testnet"
        echo "  $0 update testnet --dry-run"
        echo "  $0 verify devnet3"
        ;;
    *)
        echo -e "${RED}Unknown command: $COMMAND${NC}"
        echo "Run '$0 help' for usage information"
        exit 1
        ;;
esac

