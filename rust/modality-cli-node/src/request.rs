use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modality_node::actions;
use modality_node::logging;

use super::ephemeral_client;

#[derive(Debug, Parser)]
#[command(about = "Send a reqres request to a peer")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    config: Option<PathBuf>,

    /// Node directory containing config.json (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,

    #[clap(long)]
    target: String,

    /// Reqres path, e.g. `/contract/prefix_cert`
    #[clap(long)]
    path: String,

    /// JSON body (default `{}`)
    #[clap(long)]
    data: Option<String>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    logging::init_logging(None, Some(false), None)?;

    let mut client = ephemeral_client::open(opts.config.clone(), opts.dir.clone()).await?;
    let data = opts.data.clone().unwrap_or_else(|| "{}".to_string());
    let res = actions::request::run(
        &mut client.node,
        opts.target.clone(),
        opts.path.clone(),
        data,
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&res)?);
    if !res.ok {
        anyhow::bail!("request {} failed", opts.path);
    }
    Ok(())
}
