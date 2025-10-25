use anyhow::{Context, Result};
use clap::Parser;
use modal_networks::networks;

#[derive(Parser, Debug)]
pub struct Opts {
    /// Network name (e.g., testnet, mainnet, devnet1). Defaults to mainnet.
    #[arg(default_value = "mainnet")]
    network: String,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let network = networks::by_name(&opts.network)
        .with_context(|| format!("Network '{}' not found", opts.network))?;

    println!("\n╔═══════════════════════════════════════════════════════════════════╗");
    println!("║                     Modality Network Information                  ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝\n");

    println!("📡 Network Name:     {}", network.name);
    println!("📝 Description:      {}", network.description);
    println!("🔗 Bootstrappers:    {}", network.bootstrappers.len());

    if !network.bootstrappers.is_empty() {
        println!("\nBootstrapper Addresses:");
        println!("─────────────────────────────────────────────────────────────────────");
        for (i, addr) in network.bootstrappers.iter().enumerate() {
            println!("  {}. {}", i + 1, addr);
        }
    } else {
        println!("\n⚠️  No bootstrapper addresses configured for this network.");
    }

    println!("\n📍 DNS Record:");
    println!("─────────────────────────────────────────────────────────────────────");
    println!("  _dnsaddr.{}.modality.network", network.name);

    if !network.bootstrappers.is_empty() {
        println!("\n🔍 Query DNS records with:");
        println!("  dig +short txt _dnsaddr.{}.modality.network", network.name);
    }

    println!();

    Ok(())
}

