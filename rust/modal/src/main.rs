use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "modal")]
#[command(version = "0.1.0")]
#[command(about = "Modal CLI utility", long_about = None)]
struct Cli {}

#[tokio::main]
async fn main() -> Result<()> {
    let _cli = Cli::parse();
    
    println!("Modal CLI utility");
    println!();
    println!("For help information, run: modal --help");

    Ok(())
}

