use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use std::time::Instant;
use modality_network_node::actions;
use modality_network_node::node::Node;
use rand::Rng;

#[derive(Debug, Parser)]
#[command(about = "Ping a Modality Network node")]
pub struct Opts {
    #[clap(long)]
    config: PathBuf,

    #[clap(long)]
    target: String,

    #[clap(long, default_value = "1")]
    times: u32,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let times_to_ping = opts.times;
    let mut node = Node::from_config_filepath(opts.config.clone()).await?;
    log::info!("Running node as {:?}", node.peerid);
    node.setup().await?;
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
