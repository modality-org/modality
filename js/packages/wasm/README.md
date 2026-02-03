# @modality-dev/wasm

WASM bindings for Modality - Rust-powered parsing, verification, and model checking.

## Installation

```bash
npm install @modality-dev/wasm
```

## Building

The WASM module must be built from the Rust source:

```bash
# From the wasm package directory
npm run build

# Or manually
cd rust/modality-lang
wasm-pack build --target web --out-dir ../../js/packages/wasm/pkg
```

## Usage

```javascript
import * as modality from '@modality-dev/wasm';

// Initialize WASM (required before any other calls)
await modality.init();

// Parse a model
const model = modality.parseModel(`
  model escrow {
    states { idle, funded, complete }
    initial { idle }
    transitions {
      idle -[DEPOSIT]-> funded
      funded -[RELEASE]-> complete
    }
  }
`);

// Generate Mermaid diagram
const diagram = modality.generateMermaid(model);
console.log(diagram);
// stateDiagram-v2
//   [*] --> idle
//   idle --> funded : DEPOSIT
//   funded --> complete : RELEASE

// Parse and check formulas
const formulas = modality.parseFormulas(`
  formula {
    always(eventually(complete))
  }
`);

const result = modality.checkFormula(model, formulas[0]);
console.log(result);
// { satisfied: true, satisfying_states: ['idle', 'funded', 'complete'] }
```

## API

### Initialization

```javascript
await modality.init();
```

Must be called before using any other functions.

### Parsing

```javascript
// Parse a single model
const model = modality.parseModel(content);

// Parse multiple models
const models = modality.parseAllModels(content);

// Parse formulas
const formulas = modality.parseFormulas(content);
```

### Mermaid Diagrams

```javascript
// Basic diagram
const diagram = modality.generateMermaid(model);

// With styling
const styled = modality.generateMermaidStyled(model);

// With current state highlighted
const withState = modality.generateMermaidWithState(model);
```

### Model Checking

```javascript
// Check formula (per-graph requirement)
const result = modality.checkFormula(model, formula);

// Check formula (any-state requirement)  
const result = modality.checkFormulaAnyState(model, formula);

// Result structure:
// {
//   satisfied: boolean,
//   satisfying_states: string[],
//   counter_example?: string[]
// }
```

### ModalityParser Class

For stateful operations:

```javascript
const parser = new modality.ModalityParser();
await parser.init();

const model = parser.parseModel(content);
const diagram = parser.generateMermaid(model);
const result = parser.checkFormula(model, formula);
```

## Types

TypeScript definitions are included:

```typescript
import type { Model, Formula, ModelCheckResult } from '@modality-dev/wasm';
```

## Browser Usage

```html
<script type="module">
  import * as modality from '@modality-dev/wasm';
  
  await modality.init();
  
  const model = modality.parseModel(`
    model counter {
      states { zero, positive }
      initial { zero }
      transitions {
        zero -[INC]-> positive
        positive -[INC]-> positive
        positive -[DEC]-> zero
      }
    }
  `);
  
  console.log(modality.generateMermaid(model));
</script>
```

## Node.js Usage

```javascript
import * as modality from '@modality-dev/wasm';

await modality.init();

// Use as normal
const model = modality.parseModel(fs.readFileSync('model.modality', 'utf8'));
```

## Links

- [Modality Documentation](https://docs.modality.org)
- [Rust Source](../../rust/modality-lang/)
- [GitHub](https://github.com/modality-org/modality)
