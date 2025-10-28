use std::env;
use std::process::Command;

fn main() {
    // Try to get git info from environment variables first (set by build script)
    // If not available, try to get from git commands
    // Finally fall back to "unknown"
    
    let commit = env::var("MODAL_GIT_COMMIT").ok().unwrap_or_else(|| {
        Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout).ok()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "unknown".to_string())
            .trim()
            .to_string()
    });

    let branch = env::var("MODAL_GIT_BRANCH").ok().unwrap_or_else(|| {
        Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout).ok()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "unknown".to_string())
            .trim()
            .to_string()
    });

    // Set environment variables for use in the binary
    println!("cargo:rustc-env=GIT_COMMIT={}", commit);
    println!("cargo:rustc-env=GIT_BRANCH={}", branch);

    // Rerun if git state changes or if env vars change
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs");
    println!("cargo:rerun-if-env-changed=MODAL_GIT_BRANCH");
    println!("cargo:rerun-if-env-changed=MODAL_GIT_COMMIT");
}

