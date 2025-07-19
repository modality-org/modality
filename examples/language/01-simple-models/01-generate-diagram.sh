#!/bin/bash

# Example script demonstrating the modality model mermaid command

set -e

echo "🎨 Generating Mermaid diagrams from Modality files"
echo ""

# Check if modality CLI is available
if ! command -v modality &> /dev/null; then
    echo "❌ Error: modality CLI not found. Please build it first:"
    echo "   cd rust/modality && cargo build --release"
    echo "   export PATH=\$PATH:rust/modality/target/release"
    exit 1
fi

echo "📁 Current directory: $(pwd)"
echo "📄 Model file: simple-model.modality"
echo ""

# Generate diagram for the first model (default)
echo "🔍 Generating diagram for first model (default):"
echo "   modality model mermaid simple-model.modality"
echo ""
modality model mermaid simple-model.modality
echo ""

# Generate diagram for Model1
echo "🔍 Generating diagram for Model1:"
echo "   modality model mermaid simple-model.modality --model Model1"
echo ""
modality model mermaid simple-model.modality --model Model1
echo ""

# Generate diagram for Model2
echo "🔍 Generating diagram for Model2:"
echo "   modality model mermaid simple-model.modality --model Model2"
echo ""
modality model mermaid simple-model.modality --model Model2
echo ""

echo "✅ Done! Copy the Mermaid output above into https://mermaid.live/ to visualize" 