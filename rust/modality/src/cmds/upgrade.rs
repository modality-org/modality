use anyhow::{anyhow, Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(about = "Upgrade modality to the latest version")]
pub struct Opts {
    #[arg(long, default_value = "testnet", help = "Branch to upgrade from (testnet or mainnet)")]
    pub branch: String,
    
    #[arg(long, help = "Specific version to upgrade to (default: latest)")]
    pub version: Option<String>,
    
    #[arg(long, default_value = "http://get.modal.money", help = "Base URL for package downloads")]
    pub base_url: String,
    
    #[arg(long, help = "Force upgrade even if already on latest version")]
    pub force: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct Manifest {
    version: String,
    git_branch: String,
    git_commit: String,
    packages: Packages,
}

#[derive(Debug, Deserialize, Serialize)]
struct Packages {
    binaries: std::collections::HashMap<String, BinaryInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
struct BinaryInfo {
    name: String,
    path: String,
    platform: String,
    arch: String,
}

fn detect_platform() -> Result<String> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    
    let platform = match (os, arch) {
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("macos", "x86_64") => "darwin-x86_64",
        ("macos", "aarch64") => "darwin-aarch64",
        ("windows", "x86_64") => "windows-x86_64",
        _ => return Err(anyhow!("Unsupported platform: {} {}", os, arch)),
    };
    
    Ok(platform.to_string())
}

fn get_current_exe_path() -> Result<PathBuf> {
    env::current_exe().context("Failed to get current executable path")
}

async fn fetch_manifest(base_url: &str, branch: &str, version: Option<&str>) -> Result<Manifest> {
    let version_path = version.unwrap_or("latest");
    let manifest_url = format!("{}/{}/{}/manifest.json", base_url, branch, version_path);
    
    println!("📡 Fetching manifest from: {}", manifest_url);
    
    let response = reqwest::get(&manifest_url)
        .await
        .context("Failed to fetch manifest")?;
    
    if !response.status().is_success() {
        return Err(anyhow!("Failed to fetch manifest: HTTP {}", response.status()));
    }
    
    let manifest: Manifest = response
        .json()
        .await
        .context("Failed to parse manifest JSON")?;
    
    Ok(manifest)
}

async fn download_binary(url: &str, dest_path: &Path) -> Result<()> {
    println!("⬇️  Downloading: {}", url);
    
    let response = reqwest::get(url)
        .await
        .context("Failed to download binary")?;
    
    if !response.status().is_success() {
        return Err(anyhow!("Failed to download binary: HTTP {}", response.status()));
    }
    
    let bytes = response.bytes().await.context("Failed to read binary data")?;
    
    // Write to temporary file first
    let temp_path = dest_path.with_extension("tmp");
    fs::write(&temp_path, &bytes).context("Failed to write temporary file")?;
    
    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&temp_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&temp_path, perms)?;
    }
    
    Ok(())
}

pub async fn run(opts: &Opts) -> Result<()> {
    println!("🚀 Modality Upgrade");
    println!();
    
    // Detect current platform
    let platform = detect_platform()?;
    println!("🖥️  Platform: {}", platform);
    
    // Get current executable path
    let current_exe = get_current_exe_path()?;
    println!("📍 Current binary: {}", current_exe.display());
    
    // Fetch manifest
    let manifest = fetch_manifest(&opts.base_url, &opts.branch, opts.version.as_deref()).await?;
    println!("📦 Latest version: {}", manifest.version);
    println!("🌿 Branch: {}", manifest.git_branch);
    println!("🔖 Commit: {}", manifest.git_commit);
    println!();
    
    // Check if binary exists for this platform
    let binary_info = manifest
        .packages
        .binaries
        .get(&platform)
        .ok_or_else(|| anyhow!("No binary available for platform: {}", platform))?;
    
    // TODO: Check current version (would need to store version info in binary)
    // For now, we'll always upgrade unless --force is not set and user confirms
    
    if !opts.force {
        println!("⚠️  About to upgrade to version {}", manifest.version);
        println!("   Binary: {}/{}/{}/{}", opts.base_url, opts.branch, manifest.version, binary_info.path);
        println!();
        print!("Continue? [y/N]: ");
        std::io::Write::flush(&mut std::io::stdout())?;
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        
        if input != "y" && input != "yes" {
            println!("❌ Upgrade cancelled");
            return Ok(());
        }
    }
    
    // Download binary
    let binary_url = format!("{}/{}/{}/{}", 
        opts.base_url, 
        opts.branch, 
        opts.version.as_ref().unwrap_or(&"latest".to_string()), 
        binary_info.path
    );
    
    let temp_path = current_exe.with_extension("tmp");
    download_binary(&binary_url, &temp_path).await?;
    
    println!("✅ Downloaded successfully");
    println!();
    
    // Replace current binary
    println!("🔄 Replacing binary...");
    self_replace::self_replace(&temp_path)
        .context("Failed to replace binary")?;
    
    // Clean up temp file
    let _ = fs::remove_file(&temp_path);
    
    println!("✅ Upgrade complete!");
    println!();
    println!("🎉 Modality has been upgraded to version {}", manifest.version);
    println!("   Run 'modality --version' to verify");
    
    Ok(())
}

