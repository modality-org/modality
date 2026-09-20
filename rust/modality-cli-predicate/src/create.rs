use anyhow::{Context, Result, bail};
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
pub struct Opts {
    /// Directory to create the predicate project in
    #[arg(long)]
    dir: PathBuf,

    /// Name of the predicate (defaults to directory name)
    #[arg(long)]
    name: Option<String>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let dir = opts.dir.canonicalize().unwrap_or(opts.dir.clone());
    
    // Check if directory already exists
    if dir.exists() {
        bail!("Directory '{}' already exists", dir.display());
    }

    // Determine predicate name
    let name = opts.name.clone().unwrap_or_else(|| {
        dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-predicate")
            .to_string()
    });

    println!("Creating predicate project: {}", name);
    println!("Location: {}", dir.display());
    println!();

    // Create directory structure
    fs::create_dir_all(&dir)
        .context("Failed to create project directory")?;
    
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir)
        .context("Failed to create src directory")?;
    
    let tests_dir = dir.join("tests");
    fs::create_dir_all(&tests_dir)
        .context("Failed to create tests directory")?;

    // Generate template files
    create_cargo_toml(&dir, &name)?;
    create_lib_rs(&src_dir)?;
    create_package_json(&dir, &name)?;
    create_build_sh(&dir)?;
    create_readme(&dir, &name)?;
    create_tests(&tests_dir)?;

    println!("✓ Created Cargo.toml");
    println!("✓ Created src/lib.rs");
    println!("✓ Created package.json");
    println!("✓ Created build.sh");
    println!("✓ Created README.md");
    println!("✓ Created tests/lib.rs");
    println!();
    println!("Predicate project '{}' created successfully!", name);
    println!();
    println!("Next steps:");
    println!("  1. cd {}", dir.display());
    println!("  2. Implement your predicate logic in src/lib.rs");
    println!("  3. Run ./build.sh to compile to WASM");
    println!("  4. Upload to contract: modal contract wasm-upload --dir <contract> --wasm-file dist/{}.wasm", name.replace("-", "_"));
    println!();

    Ok(())
}

fn create_cargo_toml(dir: &Path, name: &str) -> Result<()> {
    let content = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"

[dev-dependencies]
wasm-bindgen-test = "0.3"

[profile.release]
opt-level = "z"
lto = true
"#, name);

    fs::write(dir.join("Cargo.toml"), content)
        .context("Failed to write Cargo.toml")?;
    Ok(())
}

fn create_lib_rs(src_dir: &Path) -> Result<()> {
    let content = r#"use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Input structure for predicates
/// This is passed to your evaluate function
#[derive(Debug, Deserialize)]
struct PredicateInput {
    /// The data to evaluate (your custom parameters)
    data: Value,
    /// Context information from the blockchain
    context: PredicateContext,
}

/// Context passed to predicates during evaluation
#[derive(Debug, Deserialize)]
struct PredicateContext {
    /// Contract ID being evaluated
    contract_id: String,
    /// Current block height
    block_height: u64,
    /// Current timestamp (Unix epoch)
    timestamp: u64,
}

/// Result of predicate evaluation
#[derive(Debug, Serialize)]
struct PredicateResult {
    /// Whether the predicate evaluated to true
    valid: bool,
    /// Gas consumed during execution
    gas_used: u64,
    /// Any errors encountered during evaluation
    errors: Vec<String>,
}

impl PredicateResult {
    fn success(gas_used: u64) -> Self {
        Self {
            valid: true,
            gas_used,
            errors: Vec::new(),
        }
    }

    fn failure(gas_used: u64, errors: Vec<String>) -> Self {
        Self {
            valid: false,
            gas_used,
            errors,
        }
    }

    fn error(gas_used: u64, error: String) -> Self {
        Self {
            valid: false,
            gas_used,
            errors: vec![error],
        }
    }
}

/// Main predicate evaluation function
/// 
/// This function is called by the WASM runtime with a JSON string containing
/// the input data and context. It should return a JSON string with the result.
/// 
/// TODO: Implement your predicate logic here
#[wasm_bindgen]
pub fn evaluate(input_json: &str) -> String {
    let gas_used = 25; // Base gas cost

    // Parse input
    let input: PredicateInput = match serde_json::from_str(input_json) {
        Ok(i) => i,
        Err(e) => {
            let result = PredicateResult::error(gas_used, format!("Invalid input: {}", e));
            return serde_json::to_string(&result).unwrap();
        }
    };

    // TODO: Extract your custom parameters from input.data
    // Example:
    // let my_param = match input.data.get("my_param").and_then(|v| v.as_str()) {
    //     Some(p) => p,
    //     None => {
    //         let result = PredicateResult::error(gas_used, "Missing 'my_param'".to_string());
    //         return serde_json::to_string(&result).unwrap();
    //     }
    // };

    // TODO: Access context information if needed
    // let contract_id = &input.context.contract_id;
    // let block_height = input.context.block_height;
    // let timestamp = input.context.timestamp;

    // TODO: Implement your validation logic here
    // Replace this placeholder with your actual logic
    let is_valid = false; // Change this based on your logic
    let additional_gas = 100; // Gas cost for your computation

    if is_valid {
        let result = PredicateResult::success(gas_used + additional_gas);
        serde_json::to_string(&result).unwrap()
    } else {
        let result = PredicateResult::failure(
            gas_used + additional_gas,
            vec!["Predicate validation failed".to_string()]
        );
        serde_json::to_string(&result).unwrap()
    }
}

// You can add helper functions here
// fn my_helper_function(param: &str) -> bool {
//     // ...
// }
"#;

    fs::write(src_dir.join("lib.rs"), content)
        .context("Failed to write src/lib.rs")?;
    Ok(())
}

fn create_package_json(dir: &Path, name: &str) -> Result<()> {
    let content = format!(r#"{{
  "name": "{}",
  "version": "0.1.0",
  "description": "Custom WASM predicate for Modality",
  "main": "dist/{}.js",
  "types": "dist/{}.d.ts",
  "files": [
    "dist/"
  ],
  "scripts": {{
    "build": "wasm-pack build --target web --out-dir dist",
    "build-node": "wasm-pack build --target nodejs --out-dir dist-node",
    "build-bundler": "wasm-pack build --target bundler --out-dir dist-bundler",
    "test": "wasm-pack test --headless --firefox",
    "clean": "rm -rf dist dist-node dist-bundler target"
  }},
  "keywords": [
    "modality",
    "predicate",
    "wasm",
    "webassembly"
  ],
  "license": "MIT",
  "devDependencies": {{
    "wasm-pack": "^0.12.1"
  }}
}}
"#, name, name.replace("-", "_"), name.replace("-", "_"));

    fs::write(dir.join("package.json"), content)
        .context("Failed to write package.json")?;
    Ok(())
}

fn create_build_sh(dir: &Path) -> Result<()> {
    let content = r#"#!/bin/bash
# Build script for WASM predicate

set -e

echo "Building WASM predicate..."

# Check if wasm-pack is installed
if command -v wasm-pack &> /dev/null; then
    echo "Using wasm-pack..."
    wasm-pack build --target web --out-dir dist
    
    # Rename the _bg.wasm file to remove the _bg suffix for easier uploading
    for file in dist/*_bg.wasm; do
        if [ -f "$file" ]; then
            newname="${file/_bg.wasm/.wasm}"
            mv "$file" "$newname"
            echo "✓ Renamed $(basename "$file") to $(basename "$newname")"
        fi
    done
    
    # Also rename the corresponding TypeScript definition file
    for file in dist/*_bg.wasm.d.ts; do
        if [ -f "$file" ]; then
            newname="${file/_bg.wasm.d.ts/.wasm.d.ts}"
            mv "$file" "$newname"
            echo "✓ Renamed $(basename "$file") to $(basename "$newname")"
        fi
    done
    
    echo "✓ Built with wasm-pack (output: dist/)"
else
    echo "wasm-pack not found, using cargo directly..."
    
    # Check if wasm32-unknown-unknown target is installed
    if ! rustup target list | grep -q "wasm32-unknown-unknown (installed)"; then
        echo "Installing wasm32-unknown-unknown target..."
        rustup target add wasm32-unknown-unknown
    fi
    
    # Build with cargo
    cargo build --target wasm32-unknown-unknown --release
    
    # Copy to dist directory for consistency
    mkdir -p dist
    cp target/wasm32-unknown-unknown/release/*.wasm dist/
    
    echo "✓ Built with cargo and copied to dist/"
    echo ""
    echo "Note: For full wasm-pack features, install it with:"
    echo "  cargo install wasm-pack"
fi

echo ""
echo "Build complete!"
echo "WASM file(s) ready in dist/"
ls -lh dist/*.wasm 2>/dev/null || true
"#;

    let path = dir.join("build.sh");
    fs::write(&path, content)
        .context("Failed to write build.sh")?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms)?;
    }
    
    Ok(())
}

fn create_readme(dir: &Path, name: &str) -> Result<()> {
    let content = format!(r#"# {}

Custom WASM predicate for Modality.

## What is a Predicate?

Predicates are boolean functions that can be used in Modality models to validate state transitions and create temporal logic formulas. They are compiled to WebAssembly and executed deterministically across the network.

## Project Structure

```
.
├── Cargo.toml          # Rust project configuration
├── package.json        # NPM scripts for wasm-pack
├── build.sh            # Build script
├── src/
│   └── lib.rs         # Your predicate implementation
└── tests/
    └── lib.rs         # Tests for your predicate
```

## Implementation Guide

### 1. Edit `src/lib.rs`

The main function you need to implement is `evaluate()`. It receives a JSON string with:

```json
{{
  "data": {{
    // Your custom parameters go here
  }},
  "context": {{
    "contract_id": "string",
    "block_height": 123,
    "timestamp": 1234567890
  }}
}}
```

Your predicate should:
1. Parse the input
2. Extract your custom parameters from `data`
3. Perform validation logic
4. Return a result with `valid`, `gas_used`, and `errors`

### 2. Build to WASM

```bash
# Using the build script (recommended)
./build.sh

# Or manually with wasm-pack
wasm-pack build --target web --out-dir dist

# Or with cargo directly
cargo build --target wasm32-unknown-unknown --release
```

### 3. Test Your Predicate

```bash
# Run Rust tests
cargo test

# Run WASM tests (requires wasm-pack)
wasm-pack test --headless --firefox
```

### 4. Upload to Contract

```bash
# Upload the compiled WASM module
modal contract wasm-upload \
  --dir ./my-contract \
  --wasm-file target/wasm32-unknown-unknown/release/{}.wasm \
  --module-name my_predicate \
  --gas-limit 5000000
```

### 5. Use in Modality Models

```modality
model my_model:
  part my_part:
    state1 -> state2: +my_predicate({{"param": "value"}})
```

## Example Parameters

Here are some common patterns for extracting parameters:

```rust
// String parameter
let my_string = match input.data.get("my_string").and_then(|v| v.as_str()) {{
    Some(s) => s.to_string(),
    None => return PredicateResult::error(gas_used, "Missing 'my_string'".to_string()),
}};

// Number parameter
let my_number = match input.data.get("my_number").and_then(|v| v.as_f64()) {{
    Some(n) => n,
    None => return PredicateResult::error(gas_used, "Missing 'my_number'".to_string()),
}};

// Boolean parameter
let my_bool = input.data.get("my_bool").and_then(|v| v.as_bool()).unwrap_or(false);

// Array parameter
let my_array = match input.data.get("my_array").and_then(|v| v.as_array()) {{
    Some(arr) => arr,
    None => return PredicateResult::error(gas_used, "Missing 'my_array'".to_string()),
}};
```

## Gas Metering

Your predicate should track gas usage:
- Base gas cost (parsing, setup): ~25 gas
- Computation gas: depends on your logic
- Return total gas used in the result

Gas limits are enforced by the runtime to prevent infinite loops.

## Best Practices

1. **Keep predicates simple**: Complex logic costs more gas
2. **Validate inputs early**: Return errors quickly for invalid data
3. **Be deterministic**: Same input must always produce same output
4. **Test thoroughly**: Write tests for edge cases
5. **Document parameters**: Clear documentation helps users

## Troubleshooting

### Build fails with "wasm-pack not found"
```bash
cargo install wasm-pack
```

### Build fails with "target not found"
```bash
rustup target add wasm32-unknown-unknown
```

### Runtime error: "Invalid input"
Check that your JSON matches the expected structure with `data` and `context` fields.

## Resources

- [Modality Predicates Documentation](../../docs/wasm-predicates.md)
- [Standard Predicates Examples](../../docs/standard-predicates.md)
- [WASM Integration Guide](../../docs/wasm-integration.md)
"#, name, name.replace("-", "_"));

    fs::write(dir.join("README.md"), content)
        .context("Failed to write README.md")?;
    Ok(())
}

fn create_tests(tests_dir: &Path) -> Result<()> {
    let content = r#"use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[cfg(test)]
mod tests {
    use super::*;

    // You can add regular Rust tests here
    #[test]
    fn test_placeholder() {
        // TODO: Add your tests
        assert_eq!(2 + 2, 4);
    }

    // WASM-specific tests
    #[wasm_bindgen_test]
    fn test_wasm_placeholder() {
        // TODO: Add WASM-specific tests
        // These tests run in a browser environment
        assert!(true);
    }
}
"#;

    fs::write(tests_dir.join("lib.rs"), content)
        .context("Failed to write tests/lib.rs")?;
    Ok(())
}

