# Modality Quick Reference

## Language Syntax

### Models
```modality
model ModelName:
  graph graphName:
    node1 --> node2: +property1 -property2
    node2 --> node3: +property3
```

### Formulas
```modality
formula FormulaName: <+blue> true
formula ComplexFormula: <+blue +green> [-red] phi
```

## Key Concepts

### Transition Labels
- `+property`: Transition **requires** this property
- `-property`: Transition **forbids** this property  
- No mention: Transition is **neutral** (usable for any property)

### Formula Properties
- `<+property> phi`: "Exists transition with property leading to phi"
- `[+property] phi`: "All transitions with property lead to phi"
- `<+prop1 +prop2> phi`: "Exists transition with BOTH properties"

### Satisfaction Rules
- **Per-Graph** (default): At least one state from each graph satisfies
- **Any-State**: At least one state anywhere satisfies

## CLI Commands

### Generate Mermaid Diagrams
```bash
modality model mermaid file.modality
modality model mermaid file.modality --model ModelName
```

### Check Formulas
```bash
# Named formula
modality model check file.modality --model ModelName --formula FormulaName

# Direct formula text
modality model check file.modality --model ModelName --formula-text "<+blue> true"

# Default model
modality model check file.modality --formula FormulaName
```

## Common Patterns

### Basic Properties
```modality
formula HasBlue: <+blue> true
formula NoRed: [-red] true
```

### Complex Properties
```modality
formula BlueAndGreen: <+blue +green> true
formula BlueNotRed: <+blue -red> true
```

### Multi-Step
```modality
formula BlueThenGreen: <+blue> <+green> true
```

## Examples

### Simple Model
```modality
model Test:
  graph g1:
    n1 --> n2: +blue
    n2 --> n3: +green
    n3 --> n1: +red

formula HasBlue: <+blue> true
formula HasYellow: <+yellow> true  # ✅ All transitions neutral to yellow
```

### Multi-Graph Model
```modality
model MultiGraph:
  graph g1:
    n1 --> n2: +blue
  graph g2:
    n1 --> n1: +yellow

formula CrossGraph: <+blue> <+yellow> true  # ✅ Can chain across graphs
```

## Debugging

### Formula Not Satisfied?
1. Check if transitions explicitly forbid the property
2. Remember neutral transitions are usable
3. Use CLI to see which states satisfy

### Unexpected Results?
1. Distinguish transition labels vs formula properties
2. Check per-graph vs any-state criteria
3. Start with simple formulas and build up

## File Structure
```
examples/language/01-simple-models/
├── test-check.modality      # Test models and formulas
├── 01-generate-diagram.sh  # Mermaid generation examples
└── 02-check-formulas.sh    # Formula checking examples
```

## WASM Usage (JavaScript)
```javascript
// Parse models
const models = await modality.parseModels(content);

// Check formulas
const result = await modality.checkFormula(modelJson, formulaJson);

// Generate diagrams
const mermaid = await modality.generateMermaid(modelJson);
``` 