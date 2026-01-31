use anyhow::Result;
use clap::Parser;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use zip::ZipArchive;

#[derive(Debug, Parser)]
#[command(about = "Unpack a .contract file (zip) into a directory")]
pub struct Opts {
    /// Input .contract file path
    input: PathBuf,
    
    /// Output directory (defaults to filename without .contract extension)
    #[clap(short, long)]
    output: Option<PathBuf>,
    
    /// Overwrite existing directory
    #[clap(long)]
    force: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Verify input exists
    if !opts.input.exists() {
        anyhow::bail!("File not found: {}", opts.input.display());
    }
    
    // Determine output directory
    let output = if let Some(out) = &opts.output {
        out.clone()
    } else {
        let stem = opts.input.file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("contract");
        PathBuf::from(stem)
    };
    
    // Check if output exists
    if output.exists() && !opts.force {
        anyhow::bail!(
            "Directory already exists: {}\nUse --force to overwrite",
            output.display()
        );
    }
    
    // Create output directory
    std::fs::create_dir_all(&output)?;
    
    // Open and extract zip
    let file = File::open(&opts.input)?;
    let mut archive = ZipArchive::new(file)?;
    
    let mut file_count = 0;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = output.join(file.name());
        
        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = File::create(&outpath)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            std::io::Write::write_all(&mut outfile, &buffer)?;
            file_count += 1;
        }
    }
    
    println!("✅ Unpacked {} files to {}", file_count, output.display());
    
    Ok(())
}
