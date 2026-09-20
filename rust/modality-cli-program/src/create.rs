use anyhow::{Context, Result, bail};
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
pub struct Opts {
    /// Directory to create the program project in
    #[arg(long)]
    dir: PathBuf,

    /// Name of the program (defaults to directory name)
    #[arg(long)]
    name: Option<String>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let dir = opts.dir.canonicalize().unwrap_or(opts.dir.clone());
    
    // Check if directory already exists
    if dir.exists() {
        bail!("Directory '{}' already exists", dir.display());
    }

    // Determine program name
    let name = opts.name.clone().unwrap_or_else(|| {
        dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-program")
            .to_string()
    });

    println!("Creating program project: {}", name);
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
    println!("Program project '{}' created successfully!", name);
    println!();
    println!("Next steps:");
    println!("  1. cd {}", dir.display());
    println!("  2. Implement your program logic in src/lib.rs");
    println!("  3. Run './build.sh' to compile to WASM");
    println!("  4. Upload with 'modal program upload'");
    println!("  5. Invoke with 'modal contract commit --method invoke'");

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

/// Input structure for programs
/// This is passed to your execute function
#[derive(Debug, Deserialize)]
struct ProgramInput {
    /// Custom arguments provided by the invoker
    args: Value,
    /// Context information from the blockchain
    context: ProgramContext,
}

/// Context passed to programs during execution
#[derive(Debug, Deserialize)]
struct ProgramContext {
    /// Contract ID being executed
    contract_id: String,
    /// Current block height
    block_height: u64,
    /// Current timestamp (Unix epoch)
    timestamp: u64,
    /// Public key of the user who invoked the program
    invoker: String,
}

/// Result of program execution
#[derive(Debug, Serialize)]
struct ProgramResult {
    /// Actions to include in the commit
    actions: Vec<CommitAction>,
    /// Gas consumed during execution
    gas_used: u64,
    /// Any errors encountered during execution
    errors: Vec<String>,
}

/// A commit action produced by the program
#[derive(Debug, Serialize)]
struct CommitAction {
    /// Action method (post, create, send, recv, etc.)
    method: String,
    /// Path for the action (optional, depends on method)
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    /// Value for the action
    value: Value,
}

impl ProgramResult {
    fn success(actions: Vec<CommitAction>, gas_used: u64) -> Self {
        Self {
            actions,
            gas_used,
            errors: Vec::new(),
        }
    }

    fn error(gas_used: u64, error: String) -> Self {
        Self {
            actions: Vec::new(),
            gas_used,
            errors: vec![error],
        }
    }
}

/// Main program execution function
/// 
/// This function is called by the WASM runtime with a JSON string containing
/// the input arguments and context. It should return a JSON string with the result.
/// 
/// TODO: Implement your program logic here
#[wasm_bindgen]
pub fn execute(input_json: &str) -> String {
    let gas_used = 50; // Base gas cost

    // Parse input
    let input: ProgramInput = match serde_json::from_str(input_json) {
        Ok(i) => i,
        Err(e) => {
            let result = ProgramResult::error(gas_used, format!("Invalid input: {}", e));
            return serde_json::to_string(&result).unwrap();
        }
    };

    // TODO: Extract your custom parameters from input.args
    // Example:
    // let my_param = match input.args.get("my_param").and_then(|v| v.as_str()) {
    //     Some(p) => p,
    //     None => {
    //         let result = ProgramResult::error(gas_used, "Missing 'my_param'".to_string());
    //         return serde_json::to_string(&result).unwrap();
    //     }
    // };

    // TODO: Access context information if needed
    // let contract_id = &input.context.contract_id;
    // let block_height = input.context.block_height;
    // let timestamp = input.context.timestamp;
    // let invoker = &input.context.invoker;

    // TODO: Implement your program logic here
    // Create actions based on your computation
    
    // Example: Create a simple POST action
    let actions = vec![
        CommitAction {
            method: "post".to_string(),
            path: Some("/data/result".to_string()),
            value: serde_json::json!({
                "computed_at": input.context.timestamp,
                "computed_by": "program",
                "result": "example_value"
            }),
        },
    ];

    let additional_gas = 100; // Gas cost for your computation

    let result = ProgramResult::success(actions, gas_used + additional_gas);
    serde_json::to_string(&result).unwrap()
}

// You can add helper functions here
// fn my_helper_function(param: &str) -> String {
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
  "description": "WASM program for Modality contracts",
  "scripts": {{
    "build": "wasm-pack build --target web --release"
  }}
}}
"#, name);

    fs::write(dir.join("package.json"), content)
        .context("Failed to write package.json")?;
    Ok(())
}

fn create_build_sh(dir: &Path) -> Result<()> {
    let content = r#"#!/bin/bash
set -e

echo "Building WASM program..."
wasm-pack build --target web --release

echo "✓ Build complete!"
echo "Output: pkg/*.wasm"
"#;

    let build_sh = dir.join("build.sh");
    fs::write(&build_sh, content)
        .context("Failed to write build.sh")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&build_sh)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&build_sh, perms)?;
    }

    Ok(())
}

fn create_readme(dir: &Path, name: &str) -> Result<()> {
    let content = format!(r#"# {}

A WASM program for Modality contracts.

## What are WASM Programs?

Programs are executable code stored in contracts that:
- Take input arguments
- Perform computation
- Produce commit actions (post, create, send, etc.)
- Are executed by validators during consensus

Unlike predicates (which evaluate to true/false), programs generate actions.

## Building

```bash
./build.sh
```

This compiles the Rust code to WASM using wasm-pack.

## Uploading

```bash
modal program upload pkg/{}_bg.wasm \
  --contract-id mycontract \
  --name {} \
  --gas-limit 1000000
```

## Invoking

```bash
modal contract commit \
  --dir ./mycontract \
  --method invoke \
  --path "/__programs__/{}.wasm" \
  --value '{{"args": {{"key": "value"}}}}'
```

## How It Works

1. User creates commit with "invoke" action
2. User signs the commit
3. Validators receive and validate signature
4. Validators execute program deterministically
5. Program returns actions (post, send, create, etc.)
6. Actions are merged into commit and processed
7. User's signature on invoke = indirect signature on results

## Program Structure

```rust
#[wasm_bindgen]
pub fn execute(input_json: &str) -> String {{
    // Parse input (args + context)
    // Perform computation
    // Return actions as JSON
}}
```

## Testing

Run local tests:

```bash
cargo test
```

## Gas Usage

Programs are metered to prevent infinite loops. Set appropriate gas limits when uploading.
"#, name, name, name, name);

    fs::write(dir.join("README.md"), content)
        .context("Failed to write README.md")?;
    Ok(())
}

fn create_tests(tests_dir: &Path) -> Result<()> {
    let content = r#"#[cfg(test)]
mod tests {
    // Add your tests here
    
    #[test]
    fn test_example() {
        assert_eq!(2 + 2, 4);
    }
}
"#;

    fs::write(tests_dir.join("lib.rs"), content)
        .context("Failed to write tests/lib.rs")?;
    Ok(())
}

