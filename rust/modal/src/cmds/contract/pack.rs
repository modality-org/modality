use anyhow::Result;
use clap::Parser;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;
use walkdir::WalkDir;

#[derive(Debug, Parser)]
#[command(about = "Pack a contract directory into a .contract file (zip)")]
pub struct Opts {
    /// Output .contract file path
    #[clap(short, long)]
    output: Option<PathBuf>,
    
    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let dir = opts.dir.clone().unwrap_or_else(|| std::env::current_dir().unwrap());
    
    // Verify it's a contract directory
    if !dir.join(".contract").exists() {
        anyhow::bail!("Not a contract directory: {}", dir.display());
    }
    
    // Determine output path
    let output = if let Some(out) = &opts.output {
        out.clone()
    } else {
        let dir_name = dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("contract");
        PathBuf::from(format!("{}.contract", dir_name))
    };
    
    // Create zip file
    let file = File::create(&output)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    
    // Walk the directory and add all files
    let mut file_count = 0;
    for entry in WalkDir::new(&dir) {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(&dir)?;
        
        // Skip empty path (the root directory itself)
        if relative.as_os_str().is_empty() {
            continue;
        }
        
        let relative_str = relative.to_string_lossy();
        
        if path.is_file() {
            zip.start_file(relative_str.clone(), options)?;
            let mut f = File::open(path)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
            file_count += 1;
        } else if path.is_dir() {
            zip.add_directory(format!("{}/", relative_str), options)?;
        }
    }
    
    zip.finish()?;
    
    println!("✅ Packed {} files into {}", file_count, output.display());
    
    Ok(())
}
