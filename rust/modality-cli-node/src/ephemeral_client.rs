use anyhow::Result;
use std::path::PathBuf;

use modality_node::config_resolution::load_config_with_node_dir;
use modality_node::node::Node;

/// Short-lived client: temp datastore, no listeners, no reused peer ID.
pub struct EphemeralClient {
    pub node: Node,
    _tmp: tempfile::TempDir,
}

pub async fn open(config: Option<PathBuf>, dir: Option<PathBuf>) -> Result<EphemeralClient> {
    let dir = if config.is_none() && dir.is_none() {
        Some(std::env::current_dir()?)
    } else {
        dir
    };

    let tmp = tempfile::tempdir()?;
    let mut cfg = load_config_with_node_dir(config, dir)?;
    cfg.data_dir = Some(tmp.path().join("data"));
    cfg.storage_path = None;
    cfg.listeners = Some(vec![]);
    cfg.network_config_path = None;
    cfg.passfile_path = None;

    let mut node = Node::from_config(cfg.clone()).await?;
    node.setup(&cfg).await?;
    Ok(EphemeralClient { node, _tmp: tmp })
}
