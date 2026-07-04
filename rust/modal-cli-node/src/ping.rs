use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use std::time::Instant;
use modal_node::actions;
use modal_node::node::Node;
use modal_node::config_resolution::load_config_with_node_dir;
use modal_node::logging;
use rand::Rng;

#[derive(Debug, Parser)]
#[command(about = "Ping a Modality Network node")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    config: Option<PathBuf>,

    /// Node directory containing config.json (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,

    #[clap(long)]
    target: String,

    #[clap(long, default_value = "1")]
    times: u32,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Initialize console logging for ping output
    // Use None for log_level to allow RUST_LOG env var to control verbosity
    logging::init_logging(None, Some(false), None)?;
    
    // If neither config nor dir is provided, default to current directory
    let dir = if opts.config.is_none() && opts.dir.is_none() {
        Some(std::env::current_dir()?)
    } else {
        opts.dir.clone()
    };
    
    let config = load_config_with_node_dir(opts.config.clone(), dir)?;
    
    let times_to_ping = opts.times;
    let mut node = Node::from_config(config.clone()).await?;
    log::info!("Pinging from node: {:?}", node.peerid);
    node.setup(&config).await?;
    let target = opts.target.clone();

    let start = Instant::now();

    let random_hex = generate_random_hex_string();

    log::info!("Pinging {} {} time(s)...", target, times_to_ping);
    
    for i in 0..times_to_ping {
        let path = String::from("/ping");
        let data = serde_json::json!({
            "random": random_hex
        }).to_string();
        
        let ping_start = Instant::now();
        match actions::request::run(&mut node, target.clone(), path, data).await {
            Ok(_) => {
                let ping_duration = ping_start.elapsed();
                log::info!("Ping {} successful: {:?}", i + 1, ping_duration);
            }
            Err(e) => {
                log::error!("Ping {} failed: {}", i + 1, e);
                return Err(e);
            }
        }
    }
    
    let duration = start.elapsed();
    log::info!("");
    log::info!("--- Ping Statistics ---");
    log::info!("Total pings: {}", times_to_ping);
    log::info!("Total time: {:?}", duration);
    log::info!("Average time: {:?}", duration / times_to_ping);

    Ok(())
}

fn generate_random_hex_string() -> String {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes);
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

