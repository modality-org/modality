//! Node module - Core node functionality.
//!
//! This module contains the Node struct and its implementation,
//! with helper modules for specific functionality areas.

mod helpers;

use anyhow::Result;
use futures::prelude::*;
use libp2p::gossipsub::IdentTopic;
use libp2p::request_response::OutboundRequestId;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use libp2p::gossipsub;
use libp2p::multiaddr::Protocol;
use libp2p::request_response;
use libp2p::swarm::SwarmEvent;
use libp2p::{Multiaddr, PeerId};

use modal_validator_consensus::communication::Message as ConsensusMessage;
use modal_datastore::DatastoreManager;
use modal_common::multiaddr_list::resolve_dns_multiaddrs;

use crate::config::Config;
use crate::consensus::net_comm::NetComm;
use crate::gossip;
use crate::reqres;
use crate::swarm;
use crate::constants::{
    NETWORKING_TICK_INTERVAL_SECS, SHUTDOWN_WAIT_MS, CONNECTION_WAIT_INTERVAL_SECS,
    PEER_IGNORE_INITIAL_SECS, PEER_IGNORE_MAX_EXPONENT,
};

pub use helpers::{extract_peer_id, exclude_multiaddresses_with_peerid};

/// Information about an ignored peer
#[derive(Clone, Debug)]
pub struct IgnoredPeerInfo {
    pub ignore_until: Instant,
    pub ignore_count: u32,
}

/// The main node struct that coordinates all node functionality
pub struct Node {
    pub peerid: libp2p_identity::PeerId,
    pub node_keypair: libp2p_identity::Keypair,
    pub listeners: Vec<Multiaddr>,
    pub bootstrappers: Vec<Multiaddr>,
    pub swarm: Arc<Mutex<swarm::NodeSwarm>>,
    pub datastore_manager: Arc<Mutex<DatastoreManager>>,
    pub miner_nominees: Option<Vec<String>>,
    pub hybrid_consensus: bool,
    pub run_validator: bool,
    pub network_name: String,
    pub role: String,
    pub ignored_peers: Arc<Mutex<HashMap<PeerId, IgnoredPeerInfo>>>,
    pub sync_request_tx: Option<mpsc::UnboundedSender<(PeerId, String)>>,
    pub mining_update_tx: Option<mpsc::UnboundedSender<u64>>,
    pub epoch_transition_tx: tokio::sync::broadcast::Sender<u64>,
    pub reqres_response_txs: Arc<Mutex<HashMap<OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
    pub minimum_block_timestamp: Option<i64>,
    pub fork_config: modal_observer::ForkConfig,
    pub initial_difficulty: Option<u128>,
    pub miner_hash_func: Option<String>,
    pub miner_hash_params: Option<serde_json::Value>,
    pub mining_delay_ms: Option<u64>,
    pub mining_metrics: crate::mining_metrics::SharedMiningMetrics,
    pub mining_shutdown: Option<Arc<std::sync::atomic::AtomicBool>>,
    networking_task: Option<tokio::task::JoinHandle<Result<()>>>,
    autoupgrade_task: Option<tokio::task::JoinHandle<Result<()>>>,
    status_server_task: Option<tokio::task::JoinHandle<()>>,
    status_html_writer_task: Option<tokio::task::JoinHandle<()>>,
    pub autoupgrade_config: Option<crate::autoupgrade::AutoupgradeConfig>,
    pub status_port: Option<u16>,
    pub status_html_dir: Option<PathBuf>,
    pub status_url: Option<String>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
    #[allow(dead_code)]
    consensus_rx: Option<mpsc::Receiver<ConsensusMessage>>,
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
    pub sync_trigger_tx: tokio::sync::broadcast::Sender<u64>,
}

impl Node {
    /// Create a node from a config file path
    pub async fn from_config_filepath(config_filepath: PathBuf) -> Result<Node> {
        let config = Config::from_filepath(&config_filepath)?;
        Node::from_config(config).await
    }

    /// Create a node from a Config
    pub async fn from_config(config: Config) -> Result<Node> {
        let node_keypair = config.get_libp2p_keypair().await?;
        let peerid = node_keypair.public().to_peer_id();
        let autoupgrade_config = crate::autoupgrade::AutoupgradeConfig::from_node_config(&config);
        let miner_nominees = config.miner_nominees.clone();
        let hybrid_consensus = config.hybrid_consensus.unwrap_or(false);
        let run_validator = config.run_validator.unwrap_or(false);
        let network_name = config.get_network_name();
        let role = config.get_node_role();
        let status_port = config.status_port;
        let status_html_dir = config.status_html_dir.clone();
        let status_url = config.status_url.clone();
        let minimum_block_timestamp = config.minimum_block_timestamp;
        let fork_config = config.get_fork_config();
        let initial_difficulty = config.get_initial_difficulty();
        let miner_hash_func = config.miner_hash_func.clone();
        let miner_hash_params = config.miner_hash_params.clone();
        let mining_delay_ms = config.mining_delay_ms;
        let listeners = config.listeners.clone().unwrap_or_default();
        let resolved_bootstrappers =
            resolve_dns_multiaddrs(config.bootstrappers.clone().unwrap_or_default()).await?;
        let bootstrappers = exclude_multiaddresses_with_peerid(resolved_bootstrappers, peerid);
        let swarm = swarm::create_swarm_with_status_url(node_keypair.clone(), status_url.clone()).await?;
        
        // Initialize the DatastoreManager
        let datastore_manager = helpers::initialize_datastore(&config).await?;
        
        // Load network config if provided
        if let Some(network_config_path) = config.network_config_path {
            helpers::load_network_config(&datastore_manager, network_config_path).await?;
        }
        
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        let (consensus_tx, consensus_rx) = mpsc::channel(100);
        let (sync_trigger_tx, _sync_trigger_rx) = tokio::sync::broadcast::channel(100);
        let (epoch_transition_tx, _) = tokio::sync::broadcast::channel(10);
        
        let node = Self {
            peerid,
            node_keypair,
            listeners,
            bootstrappers,
            swarm: Arc::new(Mutex::new(swarm)),
            datastore_manager,
            miner_nominees,
            hybrid_consensus,
            run_validator,
            network_name,
            role,
            ignored_peers: Arc::new(Mutex::new(HashMap::new())),
            sync_request_tx: None,
            mining_update_tx: None,
            epoch_transition_tx,
            reqres_response_txs: Arc::new(Mutex::new(HashMap::new())),
            minimum_block_timestamp,
            fork_config,
            initial_difficulty,
            miner_hash_func,
            miner_hash_params,
            mining_delay_ms,
            mining_metrics: crate::mining_metrics::create_shared_metrics(),
            mining_shutdown: None,
            networking_task: None,
            autoupgrade_task: None,
            status_server_task: None,
            status_html_writer_task: None,
            autoupgrade_config,
            status_port,
            status_html_dir,
            status_url,
            consensus_tx,
            consensus_rx: Some(consensus_rx),
            shutdown_tx,
            sync_trigger_tx,
        };
        Ok(node)
    }

    /// Get the DatastoreManager for multi-store operations
    pub fn get_datastore_manager(&self) -> Arc<Mutex<DatastoreManager>> {
        self.datastore_manager.clone()
    }

    /// Set up the node - run bootup tasks and configure swarm
    pub async fn setup(&mut self, config: &Config) -> Result<()> {
        self.run_bootup_tasks(config).await?;

        let mut swarm = self.swarm.lock().await;
        for listener in self.listeners.clone() {
            swarm.listen_on(listener.clone())?;
            swarm.add_external_address(listener.clone());
        }
        for bootstrapper in self.bootstrappers.clone() {
            if let Some(peer_id) = extract_peer_id(bootstrapper.clone()) {
                log::info!("Adding Bootstrap Peer: {peer_id:?} {bootstrapper:?}");
                swarm.add_peer_address(peer_id.clone(), bootstrapper.clone());
                swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&peer_id.clone(), bootstrapper.clone());
            } else {
                log::info!("skipping bootstrapper missing peerid: {bootstrapper:?}");
            }
        }
        Ok(())
    }

    /// Wait for peer connections
    pub async fn wait_for_connections(&mut self) -> Result<()> {
        let count = self.swarm.lock().await.connected_peers().count();
        loop {
            log::info!("connecting to peers...");
            log::info!("{}", count);
            let count = self.swarm.lock().await.connected_peers().count();
            tokio::time::sleep(Duration::from_secs(CONNECTION_WAIT_INTERVAL_SECS)).await;
            for bootstrapper in self.bootstrappers.clone() {
                log::info!("{}", bootstrapper);
                if let Some(peer_id) = extract_peer_id(bootstrapper.clone()) {
                    {
                        let mut swarm = self.swarm.lock().await;
                        swarm.add_peer_address(peer_id.clone(), bootstrapper.clone());
                        swarm
                            .behaviour_mut()
                            .kademlia
                            .add_address(&peer_id.clone(), bootstrapper.clone());
                        swarm.dial(bootstrapper.clone())?;
                    }
                }
            }
            if count > 0 {
                break;
            }
        }
        Ok(())
    }

    /// Get consensus communication interface
    pub async fn get_consensus_communication(self) -> NetComm {
        NetComm::new(self)
    }

    /// Send a request without waiting for response
    pub async fn send_request_only(
        &mut self,
        target_peer_id: PeerId,
        path: String,
        data: String,
    ) -> Result<OutboundRequestId> {
        let data_value = if data.is_empty() {
            None
        } else {
            Some(serde_json::from_str(&data)?)
        };
        
        let request = reqres::Request {
            path: path.clone().to_string(),
            data: data_value,
        };
        let req_id = {
            let mut swarm = self.swarm.lock().await;
            swarm
                .behaviour_mut()
                .reqres
                .send_request(&target_peer_id, request)
        };
        Ok(req_id)
    }

    /// Send a request and wait for response
    pub async fn send_request(
        &mut self,
        target_peer_id: PeerId,
        path: String,
        data: String,
    ) -> Result<reqres::Response> {
        let target_request_id = self.send_request_only(target_peer_id, path, data).await?;
        let res: reqres::Response;
        loop {
            let mut swarm = self.swarm.lock().await;
            futures::select!(
                event = swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(swarm::NodeBehaviourEvent::Reqres(
                        request_response::Event::Message {
                            message: request_response::Message::Response { response, request_id, .. },
                            ..
                        }
                    )) => {
                        if target_request_id == request_id {
                            res = response.clone();
                            break;
                        }
                    }
                    _ => {}
                }
            )
        }
        Ok(res)
    }

    /// Connect to a peer by multiaddr
    pub async fn connect_to_peer_multiaddr(&mut self, ma: Multiaddr) -> Result<()> {
        let mut swarm = self.swarm.lock().await;
        swarm.dial(ma.clone())?;

        let Some(Protocol::P2p(target_peer_id)) = ma.iter().last() else {
            anyhow::bail!("Provided address must end in `/p2p` and include PeerID");
        };

        loop {
            match swarm.select_next_some().await {
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    if peer_id == target_peer_id {
                        log::debug!("Connected to peer {:?}", peer_id);
                        break;
                    }
                }
                SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                    if let Some(peer_id) = peer_id {
                        log::error!("Failed to dial peer {:?}", peer_id);
                        log::error!("Error: {:?}", error);
                        anyhow::bail!("Failed to dial peer");
                    }
                }
                event => {
                    log::debug!("Other Event {:?}", event)
                }
            }
        }

        Ok(())
    }

    /// Disconnect from a peer
    pub async fn disconnect_from_peer_id(&mut self, target_peer_id: PeerId) -> Result<()> {
        let mut swarm = self.swarm.lock().await;
        let _ = swarm.disconnect_peer_id(target_peer_id);

        loop {
            match swarm.select_next_some().await {
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    if peer_id == target_peer_id {
                        log::debug!("Connection closed with peer {:?}", peer_id);
                        break;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Publish a gossip message
    pub async fn publish_gossip(&mut self, topic: String, data: String) -> Result<()> {
        let mut swarm = self.swarm.lock().await;
        swarm
            .behaviour_mut()
            .gossipsub
            .publish(IdentTopic::new(topic), data)?;
        Ok(())
    }

    /// Get inspection data about this node
    pub async fn get_inspection_data(&self, level: crate::inspection::InspectionLevel) -> Result<crate::inspection::InspectionData> {
        helpers::get_inspection_data(self, level).await
    }

    /// Shutdown the node
    pub async fn shutdown(&mut self) -> Result<()> {
        let mut swarm = self.swarm.lock().await;
        let ids: Vec<_> = swarm.connected_peers().cloned().collect();
        for peer_id in ids {
            swarm
                .disconnect_peer_id(peer_id)
                .map_err(|_| anyhow::anyhow!("Failed to disconnect from peer {}", peer_id))?;
        }
        tokio::time::sleep(Duration::from_millis(SHUTDOWN_WAIT_MS)).await;
        Ok(())
    }

    /// Wait for shutdown signal and cleanup
    pub async fn wait_for_shutdown(&mut self) -> Result<()> {
        let shutdown_tx = self.shutdown_tx.clone();
        let mining_shutdown = self.mining_shutdown.clone();
    
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
            log::info!("Received Ctrl-C, initiating shutdown...");
            
            modal_common::hash_tax::set_mining_shutdown(true);
            
            if let Some(ref flag) = mining_shutdown {
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            
            let _ = shutdown_tx.send(());
        });
    
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        shutdown_rx.recv().await?;
        log::info!("Shutdown signal received in wait_for_shutdown");
    
        if let Some(handle) = self.autoupgrade_task.take() {
            log::info!("Awaiting autoupgrade task shutdown...");
            handle.await??;
            log::info!("Autoupgrade task shutdown complete");
        }
    
        if let Some(handle) = self.networking_task.take() {
            log::info!("Awaiting networking task shutdown...");
            handle.await??;
            log::info!("Networking task shutdown complete");
        }

        if let Some(handle) = self.status_html_writer_task.take() {
            log::info!("Awaiting status HTML writer task shutdown...");
            handle.await.ok();
            log::info!("Status HTML writer task shutdown complete");
        }
    
        self.shutdown().await?;
        log::info!("Node shutdown complete");
        Ok(())
    }

    /// Start the HTTP status server
    pub async fn start_status_server(&mut self) -> Result<()> {
        if let Some(port) = self.status_port {
            log::info!("Starting HTTP status server on port {}", port);
            let handle = crate::status_server::start_status_server(
                port,
                self.peerid,
                self.datastore_manager.clone(),
                self.swarm.clone(),
                self.listeners.clone(),
                self.mining_metrics.clone(),
                self.network_name.clone(),
                self.role.clone(),
            )
            .await?;
            self.status_server_task = Some(handle);
        }
        Ok(())
    }

    /// Start the status HTML writer
    pub async fn start_status_html_writer(&mut self) -> Result<()> {
        if let Some(ref dir) = self.status_html_dir {
            log::info!("Starting status HTML writer to directory: {}", dir.display());
            let handle = crate::status_server::start_status_html_writer(
                dir.clone(),
                self.peerid,
                self.datastore_manager.clone(),
                self.swarm.clone(),
                self.listeners.clone(),
                self.mining_metrics.clone(),
                self.network_name.clone(),
                self.role.clone(),
                self.shutdown_tx.subscribe(),
            )
            .await?;
            self.status_html_writer_task = Some(handle);
        }
        Ok(())
    }

    /// Start the networking task
    pub async fn start_networking(&mut self) -> Result<()> {
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let swarm = self.swarm.clone();
        let peerid = self.peerid;

        let tick_interval = Duration::from_secs(NETWORKING_TICK_INTERVAL_SECS);
        let mut tick = futures_timer::Delay::new(tick_interval);

        let datastore_manager = self.datastore_manager.clone();
        let consensus_tx = self.consensus_tx.clone();
        let sync_request_tx = self.sync_request_tx.clone();
        let mining_update_tx = self.mining_update_tx.clone();
        let bootstrappers = self.bootstrappers.clone();
        let reqres_response_txs = self.reqres_response_txs.clone();
        let minimum_block_timestamp = self.minimum_block_timestamp;

        self.networking_task = Some(tokio::spawn(async move {
            loop {
                let mut swarm_lock = swarm.lock().await;
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        log::info!("Networking task shutting down");
                        let ids: Vec<_> = swarm_lock.connected_peers().cloned().collect();
                        for peer_id in ids {
                            swarm_lock.disconnect_peer_id(peer_id)
                                .map_err(|_| anyhow::anyhow!("Failed to disconnect from peer {}", peer_id))?;
                        }
                        tokio::time::sleep(Duration::from_millis(SHUTDOWN_WAIT_MS)).await;
                        break;
                    }
                    event = swarm_lock.select_next_some() => {
                        log::info!("{:?}", event);
                        match event {
                            SwarmEvent::NewListenAddr { address, .. } => {
                                let address_with_p2p = address
                                    .clone()
                                    .with(Protocol::P2p(peerid));
                                log::info!("Listening on {address_with_p2p:?}")
                            }
                            SwarmEvent::ConnectionEstablished { .. } => {
                                log::info!("CONNECTION ESTABLISHED");
                            },
                            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                                if let Some(peer_id) = peer_id {
                                    log::error!("Failed to dial peer {:?}", peer_id);
                                    log::error!("Error: {:?}", error);
                                }
                            }
                            SwarmEvent::Behaviour(swarm::NodeBehaviourEvent::Reqres(
                                request_response::Event::Message { message, .. },
                            )) => match message {
                                request_response::Message::Request {
                                    request,
                                    channel,
                                    ..
                                } => {
                                    log::info!("reqres request");
                                    let res = {
                                        let mgr = datastore_manager.lock().await;
                                        reqres::handle_request(request, &mgr, consensus_tx.clone()).await?
                                    };
                                    swarm_lock.behaviour_mut().reqres.send_response(channel, res)
                                        .expect("failed to respond")
                                }
                                request_response::Message::Response { request_id, response } => {
                                    log::debug!("reqres response received for request {:?}", request_id);
                                    let mut txs = reqres_response_txs.lock().await;
                                    if let Some(tx) = txs.remove(&request_id) {
                                        log::debug!("Forwarding response to caller");
                                        let _ = tx.send(response);
                                    } else {
                                        log::warn!("Received response for unknown request {:?}", request_id);
                                    }
                                }
                            }
                            SwarmEvent::Behaviour(swarm::NodeBehaviourEvent::Gossipsub(
                                gossipsub::Event::Message {
                                    propagation_source: _peer_id,
                                    message_id: _message_id,
                                    message,
                                },
                            )) => {
                                log::info!("Gossip received {:?}", message.topic.to_string());
                                gossip::handle_event(message, datastore_manager.clone(), consensus_tx.clone(), sync_request_tx.clone(), mining_update_tx.clone(), bootstrappers.clone(), minimum_block_timestamp).await?;
                            }
                            SwarmEvent::Behaviour(swarm::NodeBehaviourEvent::Identify(
                                libp2p::identify::Event::Received { peer_id, info, .. }
                            )) => {
                                log::debug!("Identify received from {:?}: agent_version={}", peer_id, info.agent_version);
                                
                                // Extract status_url from agent version string
                                // Format: "modal-node/version;status_url=https://..."
                                if let Some(status_url_part) = info.agent_version.split(';').find(|s| s.starts_with("status_url=")) {
                                    let status_url = status_url_part.strip_prefix("status_url=").map(|s| s.to_string());
                                    
                                    // Store peer info with status_url
                                    if let Some(url) = status_url {
                                        log::info!("Peer {} has status URL: {}", peer_id, url);
                                        let peer_info = modal_datastore::models::PeerInfo::with_status_url(
                                            peer_id.to_string(),
                                            Some(url)
                                        );
                                        
                                        // Store in NodeState
                                        let mgr = datastore_manager.lock().await;
                                        if let Err(e) = peer_info.save_to(mgr.node_state()).await {
                                            log::warn!("Failed to store peer info: {}", e);
                                        }
                                    }
                                }
                            }
                            SwarmEvent::Behaviour(event) => {
                                log::info!("SwarmEvent::Behaviour event {:?}", event);
                            }
                            event => {
                                log::info!("Other Node Event {:?}", event)
                            }
                        }
                    }
                    _ = &mut tick => {
                        log::debug!("tick");
                        tick = futures_timer::Delay::new(tick_interval);
                    }
                }
            }
            Ok(())
        }));

        Ok(())
    }
    
    /// Start the autoupgrade task
    pub async fn start_autoupgrade(&mut self) -> Result<()> {
        let Some(config) = self.autoupgrade_config.clone() else {
            log::debug!("Autoupgrade not configured, skipping");
            return Ok(());
        };

        if !config.enabled {
            log::debug!("Autoupgrade disabled, skipping");
            return Ok(());
        }

        let shutdown_rx = self.shutdown_tx.subscribe();
        
        self.autoupgrade_task = Some(tokio::spawn(async move {
            crate::autoupgrade::start_autoupgrade_task(config, shutdown_rx).await
        }));

        log::info!("Autoupgrade task started");
        Ok(())
    }

    /// Run bootup tasks if configured
    async fn run_bootup_tasks(&self, config: &Config) -> Result<()> {
        let bootup_config = config.get_bootup_config()?;
        
        if !bootup_config.enabled {
            log::debug!("Bootup tasks disabled, skipping");
            return Ok(());
        }

        log::info!("Running bootup tasks...");
        
        let bootup_runner = crate::bootup::BootupRunner::new(bootup_config);
        let mgr = self.datastore_manager.lock().await;
        bootup_runner.run(&mgr).await?;
        
        log::info!("Bootup tasks completed successfully");
        Ok(())
    }
    
    /// Check if a peer is currently ignored
    pub async fn is_peer_ignored(&self, peer_id: &PeerId) -> bool {
        let ignored_peers = self.ignored_peers.lock().await;
        if let Some(info) = ignored_peers.get(peer_id) {
            Instant::now() < info.ignore_until
        } else {
            false
        }
    }
    
    /// Add a peer to the ignore list with exponential backoff
    pub async fn ignore_peer(&self, peer_id: PeerId, reason: &str) {
        let mut ignored_peers = self.ignored_peers.lock().await;
        
        let (new_count, duration_secs) = if let Some(existing) = ignored_peers.get(&peer_id) {
            let new_count = existing.ignore_count + 1;
            let duration_secs = PEER_IGNORE_INITIAL_SECS * (1 << new_count.min(PEER_IGNORE_MAX_EXPONENT));
            (new_count, duration_secs)
        } else {
            (0, PEER_IGNORE_INITIAL_SECS)
        };
        
        let ignore_until = Instant::now() + Duration::from_secs(duration_secs);
        
        ignored_peers.insert(peer_id, IgnoredPeerInfo {
            ignore_until,
            ignore_count: new_count,
        });
        
        log::warn!(
            "Ignoring peer {} for {} seconds (count: {}, reason: {})",
            peer_id, duration_secs, new_count + 1, reason
        );
    }
    
    /// Clean up expired entries from the ignore list
    pub async fn cleanup_expired_ignores(&self) {
        let mut ignored_peers = self.ignored_peers.lock().await;
        let now = Instant::now();
        ignored_peers.retain(|_, info| now < info.ignore_until);
    }
}

