use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use std::time::Instant;
use modal_node::actions;
use modal_node::node::Node;
use modal_node::config_resolution::load_config_with_node_dir;
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
    // If neither config nor dir is provided, default to current directory
    let dir = if opts.config.is_none() && opts.dir.is_none() {
        Some(std::env::current_dir()?)
    } else {
        opts.dir.clone()
    };
    
    let config = load_config_with_node_dir(opts.config.clone(), dir)?;
    
    let times_to_ping = opts.times;
    let mut node = Node::from_config(config.clone()).await?;
    log::info!("Running node as {:?}", node.peerid);
    node.setup(&config).await?;
    let target = opts.target.clone();

    let start = Instant::now();

    let random_hex = generate_random_hex_string();

    for _times_pinged in 0..times_to_ping {
        let path = String::from("/ping");
        let data = serde_json::json!({
            "random": random_hex
        }).to_string();
        actions::request::run(&mut node, target.clone(), path, data).await?;
    }
    let duration = start.elapsed();
    log::info!("Time taken to ping {} times: {:?}", times_to_ping, duration);
    log::info!("Average time taken to ping: {:?}", duration / times_to_ping as u32);

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

