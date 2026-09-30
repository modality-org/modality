use anyhow::{bail, Context, Result};
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
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir).context("Failed to create project directory")?;

    let lib = name.replace('-', "_");
    write(&dir, "Cargo.toml", &cargo_toml(&name))?;
    write(&dir, "rust-toolchain.toml", RUST_TOOLCHAIN)?;
    write(&dir, "src/lib.rs", LIB_RS)?;
    write(&dir, "build.sh", &build_sh(&lib))?;
    write(&dir, "README.md", &readme(&name, &lib))?;
    write(&dir, ".gitignore", "/target\n")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let build_sh = dir.join("build.sh");
        let mut perms = fs::metadata(&build_sh)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&build_sh, perms)?;
    }

    println!("✓ Created Cargo.toml, rust-toolchain.toml, src/lib.rs, build.sh, README.md");
    println!();
    println!("Program project '{}' created successfully!", name);
    println!();
    println!("Next steps:");
    println!("  1. cd {}", dir.display());
    println!("  2. Write your program in `run` in src/lib.rs; `cargo test` runs it natively");
    println!("  3. Run './build.sh' to compile to WASM (prints the path and sha256)");
    println!("  4. Upload with 'modal program upload target/wasm32-unknown-unknown/release/{lib}.wasm'");
    println!("  5. Invoke with 'modal contract commit --method invoke'");

    Ok(())
}

fn write(dir: &Path, file: &str, content: &str) -> Result<()> {
    fs::write(dir.join(file), content).with_context(|| format!("Failed to write {file}"))
}

/// The compiler the generated project pins, as the pool and MOD emission
/// programs do: a program's sha256 depends on the compiler that built it.
const RUST_TOOLCHAIN: &str = r#"[toolchain]
channel = "1.94.1"
targets = ["wasm32-unknown-unknown"]
"#;

fn cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
publish = false

# A program builds on its own, even inside another workspace.
[workspace]

[lib]
# cdylib is the program; rlib lets `cargo test` call it natively.
crate-type = ["cdylib", "rlib"]

[dependencies]
serde_json = "1.0"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
"#
    )
}

/// A program written against the raw host ABI. The host links only
/// `env.abort`, so the template uses no `wasm-bindgen`.
const LIB_RS: &str = r#"//! A Modality program, written against the raw host ABI: the host calls
//! `alloc(len)`, writes the input JSON there, calls `execute(ptr, len)`, and
//! reads a little-endian `u32` length followed by the output JSON at the
//! returned pointer. The host links no imports except `env.abort`.
//!
//! The input is `{"args": ..., "context": ...}`. `args` is what the invoke
//! commit passed. `context` holds `contract_id`, `block_height` (the
//! sequenced prefix length, not a clock), `timestamp` (always 0), `invoker`
//! (the first signer of the invoke commit), `commit_id`, `parent_commit_id`,
//! and `state`: the contract's accepted state, keyed by path.
//!
//! The output is `{"actions": [...], "gas_used": 0, "errors": [...]}`. The
//! host meters gas itself. Any error refuses the whole commit.

use serde_json::{json, Value};

#[no_mangle]
pub extern "C" fn alloc(len: i32) -> i32 {
    let mut buf = Vec::<u8>::with_capacity(len.max(0) as usize);
    let ptr = buf.as_mut_ptr();
    core::mem::forget(buf);
    ptr as i32
}

/// # Safety
/// `ptr` and `len` must describe bytes the host wrote after calling `alloc`.
#[no_mangle]
pub unsafe extern "C" fn execute(ptr: i32, len: i32) -> i32 {
    let input = core::slice::from_raw_parts(ptr as *const u8, len.max(0) as usize);
    let out = serde_json::to_vec(&respond(input)).unwrap_or_default();
    let mut buf = Vec::with_capacity(4 + out.len());
    buf.extend_from_slice(&(out.len() as u32).to_le_bytes());
    buf.extend_from_slice(&out);
    let ptr = buf.as_ptr();
    core::mem::forget(buf);
    ptr as i32
}

/// The host's output object for one input, whether the program ran or not.
pub fn respond(input: &[u8]) -> Value {
    match serde_json::from_slice::<Value>(input)
        .map_err(|e| format!("input is not JSON: {e}"))
        .and_then(|input| run(&input["args"], &input["context"]))
    {
        Ok(actions) => json!({"actions": actions, "gas_used": 0, "errors": []}),
        Err(error) => json!({"actions": [], "gas_used": 0, "errors": [error]}),
    }
}

/// Your program: the actions it adds to the commit. Replace this.
///
/// This one posts `args.message` to `/data/message.text` and counts its
/// runs at `/data/runs.num`.
fn run(args: &Value, context: &Value) -> Result<Vec<Value>, String> {
    let message = args["message"]
        .as_str()
        .ok_or("args.message must be text")?;
    let runs = context["state"]["/data/runs.num"].as_u64().unwrap_or(0);
    Ok(vec![
        json!({"method": "post", "path": "/data/message.text", "value": message}),
        json!({"method": "post", "path": "/data/runs.num", "value": runs + 1}),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(args: Value, state: Value) -> Value {
        let input = json!({"args": args, "context": {"state": state}});
        respond(input.to_string().as_bytes())
    }

    #[test]
    fn posts_the_message_and_counts() {
        let out = call(json!({"message": "hi"}), json!({"/data/runs.num": 2}));
        assert_eq!(out["errors"], json!([]));
        assert_eq!(out["actions"][0]["value"], "hi");
        assert_eq!(out["actions"][1]["value"], 3);
    }

    #[test]
    fn refuses_a_missing_message() {
        let out = call(json!({}), json!({}));
        assert_eq!(out["actions"], json!([]));
        assert_eq!(out["errors"], json!(["args.message must be text"]));
    }
}
"#;

fn build_sh(lib: &str) -> String {
    format!(
        r#"#!/usr/bin/env bash
# Build the program with the pinned toolchain. Prints the wasm path and its
# sha256. The bytes depend on the host that built them (macOS or Linux,
# arm64 or amd64); build in one pinned image if others must reproduce them.
set -euo pipefail
cd "$(dirname "$0")"

# With rust-src installed, std's paths point into the local toolchain;
# without it, at rustc's own /rustc/<commit>. Map the first to the second.
SYSROOT=$(rustc --print sysroot)
COMMIT=$(rustc -vV | sed -n 's/^commit-hash: //p')
export RUSTFLAGS="--remap-path-prefix=$SYSROOT/lib/rustlib/src/rust=/rustc/$COMMIT --remap-path-prefix=${{CARGO_HOME:-$HOME/.cargo}}=/cargo --remap-path-prefix=$(pwd)=/src"
cargo build -q --release --target wasm32-unknown-unknown
WASM="$(pwd)/target/wasm32-unknown-unknown/release/{lib}.wasm"

if command -v sha256sum >/dev/null; then
    SHA=$(sha256sum "$WASM" | cut -d' ' -f1)
else
    SHA=$(shasum -a 256 "$WASM" | cut -d' ' -f1)
fi
echo "$WASM $SHA"
"#
    )
}

fn readme(name: &str, lib: &str) -> String {
    format!(
        r#"# {name}

A Modality program. An `invoke` commit runs it on the sequencer; the actions
it returns join that commit and are checked against the contract's rules
like any other.

## Test

```bash
cargo test
```

The tests call `respond` natively with the same JSON the host passes.

## Build

```bash
./build.sh
```

Prints `target/wasm32-unknown-unknown/release/{lib}.wasm` and its sha256.

## Upload

```bash
modal program upload target/wasm32-unknown-unknown/release/{lib}.wasm   --dir ./mycontract   --name {name}   --gas-limit 1000000
```

## Invoke

```bash
modal contract commit   --dir ./mycontract   --method invoke   --path "/__programs__/{name}.wasm"   --value '{{"args": {{"message": "hello"}}}}'
```

## The ABI

The host calls `alloc(len)`, writes the input JSON there, and calls
`execute(ptr, len)`. `execute` returns a pointer to a little-endian `u32`
length followed by the output JSON. The host links only `env.abort`, so
`wasm-bindgen` output does not run.

Input: `{{"args": ..., "context": {{"contract_id", "block_height",
"timestamp", "invoker", "commit_id", "parent_commit_id", "state"}}}}`.
`timestamp` is always 0 and `block_height` is the sequenced prefix length:
a program sees no clock.

Output: `{{"actions": [...], "gas_used": 0, "errors": [...]}}`. Any error
refuses the whole commit.
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_template_uses_the_raw_host_abi() {
        assert!(LIB_RS.contains("pub extern \"C\" fn alloc(len: i32) -> i32"));
        assert!(LIB_RS.contains("pub unsafe extern \"C\" fn execute(ptr: i32, len: i32) -> i32"));
        assert!(!LIB_RS.contains("wasm_bindgen"));
        assert!(!cargo_toml("p").contains("wasm-bindgen"));
        assert!(build_sh("my_prog").contains("release/my_prog.wasm"));
    }
}
