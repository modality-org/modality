# Mermaid Diagram Generation

This example demonstrates how to use the Modality CLI to generate Mermaid diagrams from `.modality` files and check formulas against models.

## Files

- `simple-model.modality` - A simple Modality model with two parts
- `test-check.modality` - Test models and formulas for model checking
- `01-generate-diagram.sh` - Script to generate Mermaid diagrams
- `02-check-formulas.sh` - Script to demonstrate formula checking

## Usage

### Mermaid Diagrams

```bash
# Generate diagram for the first model (default)
./01-generate-diagram.sh

# Generate diagram for a specific model
modality model mermaid simple-model.modality --model Model1
modality model mermaid simple-model.modality --model Model2
```

### Formula Checking

```bash
# Check a named formula against a model
modality model check test-check.modality --model TestModel1 --formula FormulaBlue

# Check a formula text directly
modality model check test-check.modality --model TestModel2 --formula-text "<+blue> true"

# Check with default model (first model in file)
modality model check test-check.modality --formula FormulaGreen
```

## Output

### Mermaid Diagrams
The script will output Mermaid diagram code that can be rendered in:
- Mermaid Live Editor: https://mermaid.live/
- GitHub markdown files
- Documentation tools that support Mermaid

### Formula Checking
The check command provides:
- Formula satisfaction status (per-graph and any-state criteria)
- List of satisfying states
- Formula expression details 