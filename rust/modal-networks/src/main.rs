use anyhow::Result;
use clap::{Parser, Subcommand};
use modal_networks::{dns::DnsManager, networks};

#[derive(Parser)]
#[command(name = "modal-networks")]
#[command(about = "Manage Modality network configurations and DNS records", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available networks
    List,
    
    /// Show information about a specific network
    Show {
        /// Network name (e.g., testnet, mainnet, devnet1)
        network: String,
    },
    
    /// Update DNS records for networks
    UpdateDns {
        /// Specific network to update (if not provided, updates all)
        #[arg(short, long)]
        network: Option<String>,
        
        /// Dry run - show what would be updated without making changes
        #[arg(short, long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List => {
            println!("Available Modality Networks:\n");
            for network in networks::all() {
                println!("  {} - {}", network.name, network.description);
                println!("    Bootstrappers: {}", network.bootstrappers.len());
            }
        }
        
        Commands::Show { network } => {
            match networks::by_name(&network) {
                Some(net) => {
                    println!("Network: {}", net.name);
                    println!("Description: {}", net.description);
                    println!("\nBootstrappers:");
                    for (i, addr) in net.bootstrappers.iter().enumerate() {
                        println!("  {}: {}", i + 1, addr);
                    }
                }
                None => {
                    eprintln!("Network '{}' not found", network);
                    eprintln!("Available networks: devnet1, devnet2, devnet3, devnet5, testnet, mainnet");
                    std::process::exit(1);
                }
            }
        }
        
        Commands::UpdateDns { network, dry_run } => {
            if dry_run {
                let networks_to_update = if let Some(name) = network {
                    vec![networks::by_name(&name)
                        .ok_or_else(|| anyhow::anyhow!("Network '{}' not found", name))?]
                } else {
                    networks::all()
                };

                println!("Dry run - would update the following networks:\n");
                for net in networks_to_update {
                    println!("Network: {}", net.name);
                    println!("  Record: _dnsaddr.{}.modality.network", net.name);
                    println!("  Values:");
                    for addr in &net.bootstrappers {
                        println!("    dnsaddr={}", addr);
                    }
                    println!();
                }
            } else {
                println!("Initializing DNS manager...");
                let dns_manager = DnsManager::new().await?;

                if let Some(name) = network {
                    let net = networks::by_name(&name)
                        .ok_or_else(|| anyhow::anyhow!("Network '{}' not found", name))?;
                    dns_manager.set_network_records(&net).await?;
                } else {
                    dns_manager.update_all_networks(&networks::all()).await?;
                }

                println!("\nDNS records updated successfully!");
                println!("\nYou can verify with:");
                println!("  dig +short txt _dnsaddr.testnet.modality.network");
            }
        }
    }

    Ok(())
}

