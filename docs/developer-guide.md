# Modality Developer Guide

This guide explains the implementation details, architecture, and how to extend the Modality language and tools.

## Architecture Overview

### Core Components

```
modality-lang/           # Core language implementation
├── src/
│   ├── ast.rs          # Abstract Syntax Tree definitions
│   ├── grammar.lalrpop # LALRPOP grammar specification
│   ├── model_checker.rs # Model checking implementation
│   ├── mermaid.rs      # Mermaid diagram generation
│   └── wasm.rs         # WebAssembly bindings

modality/                # CLI application
├── src/cmds/
│   ├── mermaid.rs      # Mermaid CLI command
│   └── check.rs        # Formula checking CLI command
```

### Data Flow

1. **Parsing**: `.modality` files → LALRPOP parser → AST
2. **Model Checking**: AST + Formula → ModelChecker → Results
3. **Output**: Results → CLI output / Mermaid diagrams / WASM

## Key Implementation Details

### AST Structure

The Abstract Syntax Tree represents parsed Modality constructs:

```rust
pub struct Model {
    pub name: String,
    pub parts: Vec<Part>,
}

pub struct Part {
    pub name: String,
    pub transitions: Vec<Transition>,
}

pub struct Transition {
    pub from: String,
    pub to: String,
    pub properties: Vec<Property>,
}

pub struct Property {
    pub sign: PropertySign,  // Plus or Minus
    pub name: String,
}

pub struct Formula {
    pub name: String,
    pub expression: FormulaExpr,
}

pub enum FormulaExpr {
    True,
    False,
    And(Box<FormulaExpr>, Box<FormulaExpr>),
    Or(Box<FormulaExpr>, Box<FormulaExpr>),
    Not(Box<FormulaExpr>),
    Diamond(Vec<Property>, Box<FormulaExpr>),  // <properties> phi
    Box(Vec<Property>, Box<FormulaExpr>),      // [properties] phi
}
```

### Model Checker Semantics

The model checker implements the core semantics:

#### Property Satisfaction
```rust
fn transition_satisfies_properties(&self, transition: &Transition, properties: &[Property]) -> bool {
    properties.iter().all(|property| {
        // Check if transition explicitly has this property
        let has_explicit = transition.properties.iter().any(|p| p == property);
        if has_explicit {
            return true;
        }
        
        // If transition doesn't mention this property at all, it's usable
        let property_name = &property.name;
        let mentions_property = transition.properties.iter().any(|p| p.name == *property_name);
        !mentions_property
    })
}
```

#### Satisfaction Criteria
- **Per-Part**: At least one state from each part satisfies the formula
- **Any-State**: At least one state anywhere satisfies the formula

### Grammar Specification

The LALRPOP grammar defines the language syntax:

```lalrpop
// Top-level items
TopLevelItem: TopLevelItem = {
    Model,
    Formula,
};

// Model definition
Model: Model = {
    "model" <name:IDENT> ":" <graphs:GraphList> => {
        Model { name, graphs, state: None }
    },
};

// Formula definition  
Formula: Formula = {
    "formula" <name:IDENT> ":" <expr:FormulaExpr> => {
        Formula { name, expression: expr }
    },
};

// Modal operators with property lists
FormulaAtom: FormulaExpr = {
    "<" <properties:PropertyList> ">" <expr:FormulaAtom> => {
        FormulaExpr::Diamond(properties, Box::new(expr))
    },
    "[" <properties:PropertyList> "]" <expr:FormulaAtom> => {
        FormulaExpr::Box(properties, Box::new(expr))
    },
};
```

## Extending the Language

### Adding New Formula Operators

1. **Update AST** (`ast.rs`):
```rust
pub enum FormulaExpr {
    // ... existing variants
    NewOperator(Box<FormulaExpr>),
}
```

2. **Update Grammar** (`grammar.lalrpop`):
```lalrpop
FormulaAtom: FormulaExpr = {
    // ... existing rules
    "new" <expr:FormulaAtom> => {
        FormulaExpr::NewOperator(Box::new(expr))
    },
};
```

3. **Update Model Checker** (`model_checker.rs`):
```rust
fn evaluate_formula(&self, expr: &FormulaExpr) -> Vec<State> {
    match expr {
        // ... existing cases
        FormulaExpr::NewOperator(sub_expr) => {
            // Implement semantics for new operator
            self.evaluate_new_operator(sub_expr)
        }
    }
}
```

### Adding New CLI Commands

1. **Create Command Module** (`modality/src/cmds/new_command.rs`):
```rust
use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Opts {
    pub input: String,
    #[arg(short, long)]
    pub option: Option<String>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Implementation
    Ok(())
}
```

2. **Register Command** (`modality/src/main.rs`):
```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands
    NewCommand(cmds::new_command::Opts),
}

// In main function:
Commands::NewCommand(opts) => cmds::new_command::run(opts).await?,
```

### Adding WASM Bindings

1. **Update WASM Module** (`modality-lang/src/wasm.rs`):
```rust
#[wasm_bindgen]
impl ModalityParser {
    pub fn new_function(&self, input: &str) -> Result<JsValue, JsValue> {
        // Implementation
        wasm_bindgen::JsValue::from_serde(&result)
            .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))
    }
}

#[wasm_bindgen]
pub fn new_standalone_function(input: &str) -> Result<JsValue, JsValue> {
    // Implementation
}
```

## Testing

### Unit Tests

Test individual components:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_satisfaction() {
        let transition = Transition::new("n1".to_string(), "n2".to_string());
        transition.add_property(Property::new(PropertySign::Plus, "blue".to_string()));
        
        let property = Property::new(PropertySign::Plus, "blue".to_string());
        assert!(transition_satisfies_property(&transition, &property));
    }
}
```

### Integration Tests

Test end-to-end functionality:

```