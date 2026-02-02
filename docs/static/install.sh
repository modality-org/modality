#!/bin/sh

# Exit on errors
set -e

echo "Checking for Rust installation..."

# Check if Rust is installed
if ! command -v rustc >/dev/null 2>&1; then
    printf "Rust not found. Installing Rust...\n"
    
    # Check if curl is installed
    if ! command -v curl >/dev/null 2>&1; then
        printf "curl is required but not installed. Please install curl first.\n"
        exit 1
    fi
    
    # Install Rust using rustup
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    
    # Source the cargo environment
    . "$HOME/.cargo/env"
    
    printf "Rust installation completed!\n"
else
    printf "Rust is already installed\n"
fi

# Verify Rust installation
rustc --version
cargo --version

printf "Installing modality...\n"

# Install modality using cargo
cargo install modality

# Verify installation
if command -v modality >/dev/null 2>&1; then
    printf "Successfully installed modality!\n"
    modality --version
else
    printf "Failed to install modality. Please check the error messages above.\n"
    exit 1
fi
