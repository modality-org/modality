#!/bin/bash

# Example script demonstrating the modality model check command

set -e

echo "🔍 Checking formulas against Modality models"
echo ""

# Check if modality CLI is available
if ! command -v modality &> /dev/null; then
    echo "❌ Error: modality CLI not found. Please build it first:"
    echo "   cd rust/modality && cargo build --release"
    echo "   export PATH=\$PATH:rust/modality/target/release"
    exit 1
fi

echo "📁 Current directory: $(pwd)"
echo "📄 Test file: test-check.modality"
echo ""

# Test 1: Check a named formula against a specific model
echo "🔍 Test 1: Check FormulaBlue against TestModel1"
echo "   modality model check test-check.modality --model TestModel1 --formula FormulaBlue"
echo ""
modality model check test-check.modality --model TestModel1 --formula FormulaBlue
echo ""

# Test 2: Check a formula text directly
echo "🔍 Test 2: Check formula text against TestModel2"
echo "   modality model check test-check.modality --model TestModel2 --formula-text '<+yellow> true'"
echo ""
modality model check test-check.modality --model TestModel2 --formula-text "<+yellow> true"
echo ""

# Test 3: Check with default model
echo "🔍 Test 3: Check FormulaGreen with default model"
echo "   modality model check test-check.modality --formula FormulaGreen"
echo ""
modality model check test-check.modality --formula FormulaGreen
echo ""

# Test 4: Check a complex formula
echo "🔍 Test 4: Check complex formula with multiple properties"
echo "   modality model check test-check.modality --model TestModel2 --formula-text '<+blue +green> true'"
echo ""
modality model check test-check.modality --model TestModel2 --formula-text "<+blue +green> true"
echo ""

echo "✅ Done! The check command shows both per-graph and any-state satisfaction criteria" 