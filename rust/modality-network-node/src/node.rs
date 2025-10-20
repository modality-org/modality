use anyhow::Result;
use futures::prelude::*;
use libp2p::gossipsub::IdentTopic;
use libp2p::request_response::OutboundRequestId;
use modality_network_consensus::runner::create_runner_props_from_datastore;
use modality_network_consensus::runner::ConsensusRunner;
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

use modality_network_consensus::communication::Message as ConsensusMessage;
use modality_network_datastore::NetworkDatastore;
use modality_utils::multiaddr_list::resolve_dns_multiaddrs;

use crate::config::Config;
use crate::consensus::net_comm::NetComm;
use crate::consensus::node_communication::NodeCommunication;
use crate::gossip;
use crate::reqres;
use crate::swarm;

/// Information about an ignored peer
#[derive(Clone, Debug)]
pub struct IgnoredPeerInfo {
    pub ignore_until: Instant,
    pub ignore_count: u32,
}

pub struct Node {
    pub peerid: libp2p_identity::PeerId,
    pub node_keypair: libp2p_identity::Keypair,
    pub listeners: Vec<Multiaddr>,
    pub bootstrappers: Vec<Multiaddr>,
    pub swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    pub datastore: Arc<Mutex<NetworkDatastore>>,
    pub miner_nominees: Option<Vec<String>>,
    pub ignored_peers: Arc<Mutex<HashMap<PeerId, IgnoredPeerInfo>>>,
    pub sync_request_tx: Option<mpsc::UnboundedSender<(PeerId, String)>>, // Set later in miner run
    pub mining_update_tx: Option<mpsc::UnboundedSender<u64>>, // Set in miner run to notify of chain tip updates
    consensus_runner: Option<modality_network_consensus::runner::Runner>,
    networking_task: Option<tokio::task::JoinHandle<Result<()>>>,
    consensus_task: Option<tokio::task::JoinHandle<Result<()>>>,
    autoupgrade_task: Option<tokio::task::JoinHandle<Result<()>>>,
    status_server_task: Option<tokio::task::JoinHandle<()>>,
    pub autoupgrade_config: Option<crate::autoupgrade::AutoupgradeConfig>,
    pub status_port: Option<u16>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
    consensus_rx: Option<mpsc::Receiver<ConsensusMessage>>,
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
    pub sync_trigger_tx: tokio::sync::broadcast::Sender<u64>,
}

impl Node {
    pub async fn from_config_filepath(config_filepath: PathBuf) -> Result<Node> {
        let config = Config::from_filepath(&config_filepath)?;
        Node::from_config(config).await
    }

    pub async fn from_config(config: Config) -> Result<Node> {
        let node_keypair = config.get_libp2p_keypair().await?;
        let peerid = node_keypair.public().to_peer_id();
        let autoupgrade_config = crate::autoupgrade::AutoupgradeConfig::from_node_config(&config);
        let listeners = config.listeners.unwrap_or_default();
        let resolved_bootstrappers =
            resolve_dns_multiaddrs(config.bootstrappers.unwrap_or_default()).await?;
        let bootstrappers = exclude_multiaddresses_with_peerid(resolved_bootstrappers, peerid);
        let swarm = crate::swarm::create_swarm(node_keypair.clone()).await?;
        let datastore = if let Some(storage_path) = config.storage_path {
            Arc::new(Mutex::new(NetworkDatastore::create_in_directory(
                &storage_path,
            )?))
        } else {
            Arc::new(Mutex::new(NetworkDatastore::create_in_memory()?))
        };
        if let Some(network_config_path) = config.network_config_path {
            let config_str = std::fs::read_to_string(network_config_path)?;
            let network_config = serde_json::from_str(&config_str)?;
            datastore
                .lock()
                .await
                .load_network_config(&network_config)
                .await?;
        }
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        let (consensus_tx, consensus_rx) = mpsc::channel(100);
        let (sync_trigger_tx, _sync_trigger_rx) = tokio::sync::broadcast::channel(100);
        let miner_nominees = config.miner_nominees.clone();
        let status_port = config.status_port;
        let node = Self {
            peerid,
            node_keypair,
            listeners,
            bootstrappers,
            swarm: Arc::new(Mutex::new(swarm)),
            datastore,
            miner_nominees,
            ignored_peers: Arc::new(Mutex::new(HashMap::new())),
            sync_request_tx: None, // Will be set in miner run()
            mining_update_tx: None, // Will be set in miner run()
            networking_task: None,
            consensus_task: None,
            autoupgrade_task: None,
            status_server_task: None,
            autoupgrade_config,
            status_port,
            consensus_tx,
            consensus_rx: Some(consensus_rx),
            shutdown_tx,
            consensus_runner: None,
            sync_trigger_tx,
        };
        Ok(node)
    }

    pub async fn setup(&mut self, config: &Config) -> Result<()> {
        // Run bootup tasks if configured
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

    pub async fn wait_for_connections(&mut self) -> Result<()> {
        let count = self.swarm.lock().await.connected_peers().count();
        loop {
            log::info!("connecting to peers...");
            log::info!("{}", count);
            let count = self.swarm.lock().await.connected_peers().count();
            tokio::time::sleep(Duration::from_millis(1000 * 5)).await;
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

    pub async fn get_consensus_communication(self) -> NetComm {
        NetComm::new(self)
    }

    pub async fn send_request_only(
        &mut self,
        target_peer_id: PeerId,
        path: String,
        data: String,
    ) -> Result<OutboundRequestId> {
        let request = reqres::Request {
            path: path.clone().to_string(),
            data: Some(serde_json::json!(data.clone())),
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

    pub async fn connect_to_peer_multiaddr(&mut self, ma: Multiaddr) -> Result<()> {
        let mut swarm = self.swarm.lock().await;
        swarm.dial(ma.clone())?;

        let Some(libp2p::multiaddr::Protocol::P2p(target_peer_id)) = ma.iter().last() else {
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

    pub async fn publish_gossip(&mut self, topic: String, data: String) -> Result<()> {
        let mut swarm = self.swarm.lock().await;
        swarm
            .behaviour_mut()
            .gossipsub
            .publish(IdentTopic::new(topic), data)?;
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        let mut swarm = self.swarm.lock().await;
        let ids: Vec<_> = swarm.connected_peers().cloned().collect();
        for peer_id in ids {
            swarm
                .disconnect_peer_id(peer_id)
                .map_err(|_| anyhow::anyhow!("Failed to disconnect from peer {}", peer_id))?;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    pub async fn wait_for_shutdown(&mut self) -> Result<()> {
        let shutdown_tx = self.shutdown_tx.clone();
    
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
            log::info!("Received Ctrl-C, initiating shutdown...");
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
    
        if let Some(handle) = self.consensus_task.take() {
            log::info!("Awaiting consensus task shutdown...");
            handle.await??;
            log::info!("Consensus task shutdown complete");
        }
    
        if let Some(handle) = self.networking_task.take() {
            log::info!("Awaiting networking task shutdown...");
            handle.await??;
            log::info!("Networking task shutdown complete");
        }
    
        self.shutdown().await?;
        log::info!("Node shutdown complete");
        Ok(())
    }

    pub async fn start_status_server(&mut self) -> Result<()> {
        if let Some(port) = self.status_port {
            log::info!("Starting HTTP status server on port {}", port);
            let handle = crate::status_server::start_status_server(
                port,
                self.peerid,
                self.datastore.clone(),
                self.swarm.clone(),
                self.listeners.clone(),
            )
            .await?;
            self.status_server_task = Some(handle);
        }
        Ok(())
    }

    pub async fn start_networking(&mut self) -> Result<()> {
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let swarm = self.swarm.clone();
        let peerid = self.peerid;

        let tick_interval: Duration = Duration::from_secs(15);
        let mut tick = futures_timer::Delay::new(tick_interval);

        let datastore = self.datastore.clone();
        let consensus_tx = self.consensus_tx.clone();
        let sync_request_tx = self.sync_request_tx.clone();
        let mining_update_tx = self.mining_update_tx.clone();
        let bootstrappers = self.bootstrappers.clone();

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
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        break;
                    }
                    event = swarm_lock.select_next_some() => {
                        log::info!("{:?}", event);
                        match event {
                            SwarmEvent::NewListenAddr { address, .. } => {
                                let address_with_p2p = address
                                    .clone()
                                    .with(libp2p::multiaddr::Protocol::P2p(peerid));
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
                            SwarmEvent::Behaviour(crate::swarm::NodeBehaviourEvent::Reqres(
                                request_response::Event::Message { message, .. },
                            )) => match message {
                                request_response::Message::Request {
                                    request,
                                    channel,
                                    ..
                                } => {
                                    log::info!("reqres request");
                                    let res = {
                                        let mut datastore = datastore.lock().await;
                                        crate::reqres::handle_request(request, &mut *datastore, consensus_tx.clone()).await?
                                    };
                                    swarm_lock.behaviour_mut().reqres.send_response(channel, res)
                                        .expect("failed to respond")
                                }
                                request_response::Message::Response { .. } => {
                                    log::info!("reqres response")
                                }
                            }
                            SwarmEvent::Behaviour(crate::swarm::NodeBehaviourEvent::Gossipsub(
                                gossipsub::Event::Message {
                                    propagation_source: _peer_id,
                                    message_id: _message_id,
                                    message,
                                },
                            )) => {
                                log::info!("Gossip received {:?}", message.topic.to_string());
                                gossip::handle_event(message, datastore.clone(), consensus_tx.clone(), sync_request_tx.clone(), mining_update_tx.clone(), bootstrappers.clone()).await?;
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

        self.shutdown().await?;

        Ok(())
    }
    pub async fn start_consensus(&mut self) -> Result<()> {
        let mut runner_props = create_runner_props_from_datastore(self.datastore.clone()).await?;
        runner_props.peerid = Some(self.peerid.to_string());
        runner_props.keypair = Some(modality_utils::keypair::Keypair::from_libp2p_keypair(
            self.node_keypair.clone(),
        )?);
        runner_props.communication = Some(Arc::new(Mutex::new(NodeCommunication {
            swarm: self.swarm.clone(),
            consensus_tx: self.consensus_tx.clone(),
        })));
        let runner = modality_network_consensus::runner::Runner::create(runner_props);
        self.consensus_runner = Some(runner.clone());
    
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        log::info!("Consensus task started and listening for shutdown signal");
        let mut consensus_rx = self
            .consensus_rx
            .take()
            .expect("Consensus receiver should be available");
    
        let token = Arc::new(tokio_util::sync::CancellationToken::new());
        let runner = Arc::new(Mutex::new(runner));
        let runner_clone = runner.clone();
        let token_clone = token.clone();
    
        let (round_tx, mut round_rx) = tokio::sync::mpsc::channel::<()>(1);
    
        self.consensus_task = Some(tokio::spawn(async move {
            let mut round_handle: Option<tokio::task::JoinHandle<()>> = None;
    
            if !token_clone.is_cancelled() {
                log::info!("Starting initial consensus round");
                let round_tx_clone = round_tx.clone();
                let runner_clone_inner = runner_clone.clone();
                let token_inner = token_clone.clone();
                round_handle = Some(tokio::spawn(async move {
                    let mut runner_lock = runner_clone_inner.lock().await;
                    if let Ok(_) = runner_lock.run_round(Some(token_inner)).await {
                        let _ = round_tx_clone.send(()).await;
                    }
                }));
            } else {
                log::warn!("Consensus task started but token already cancelled");
            }
    
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        log::info!("Shutdown signal received in consensus task");
                        token.cancel();
                        log::info!("Cancellation token triggered");
                        if let Some(handle) = round_handle.take() {
                            log::info!("Aborting round handle");
                            handle.abort();
                            if let Err(e) = handle.await {
                                log::warn!("Round handle await failed: {:?}", e);
                            }
                        }
                        break;
                    }
                    Some(msg) = consensus_rx.recv() => {
                        log::info!("Received consensus message: {:?}", msg);
                        let runner = runner_clone.lock().await;
                        match msg {
                            ConsensusMessage::DraftBlock { from: _, to: _, block } => {
                                let _ = runner.on_receive_draft_block(&block).await;
                            }
                            ConsensusMessage::BlockAck { from: _, to: _, ack } => {
                                let _ = runner.on_receive_block_ack(&ack).await;
                            }
                            ConsensusMessage::BlockLateAck { from: _, to: _, ack } => {
                                let _ = runner.on_receive_block_late_ack(&ack).await;
                            }
                            ConsensusMessage::CertifiedBlock { from: _, to: _, block } => {
                                let _ = runner.on_receive_certified_block(&block).await;
                            }
                        }
                    }
                    Some(_) = round_rx.recv() => {
                        log::info!("Round completed, starting new round");
                        if token_clone.is_cancelled() {
                            log::info!("Token cancelled, stopping consensus");
                            break;
                        }
                        let round_tx_clone = round_tx.clone();
                        let runner_clone_inner = runner_clone.clone();
                        let token_inner = token_clone.clone();
                        round_handle = Some(tokio::spawn(async move {
                            let mut runner_lock = runner_clone_inner.lock().await;
                            if let Ok(_) = runner_lock.run_round(Some(token_inner)).await {
                                let _ = round_tx_clone.send(()).await;
                            }
                        }));
                    }
                }
            }
            log::info!("Consensus task fully terminated");
            Ok(())
        }));
    
        Ok(())
    }
    
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
        
        // Create and run bootup tasks
        let bootup_runner = crate::bootup::BootupRunner::new(bootup_config);
        let datastore = self.datastore.lock().await;
        bootup_runner.run(&datastore).await?;
        
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
    /// Starts at 1 minute and doubles each time
    pub async fn ignore_peer(&self, peer_id: PeerId, reason: &str) {
        let mut ignored_peers = self.ignored_peers.lock().await;
        
        let (new_count, duration_secs) = if let Some(existing) = ignored_peers.get(&peer_id) {
            let new_count = existing.ignore_count + 1;
            let duration_secs = 60 * (1 << new_count.min(10)); // Cap at ~17 hours (2^10 minutes)
            (new_count, duration_secs)
        } else {
            (0, 60) // First time: 1 minute
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

fn extract_peer_id(multiaddr: Multiaddr) -> Option<PeerId> {
    let protocols: Vec<libp2p::multiaddr::Protocol> = multiaddr.iter().collect();
    let last_protocol = protocols.last()?;

    match last_protocol {
        Protocol::P2p(peer_id) => Some(peer_id.clone()),
        _ => None,
    }
}

fn exclude_multiaddresses_with_peerid(ma: Vec<Multiaddr>, peerid: PeerId) -> Vec<Multiaddr> {
    ma.into_iter()
        .filter(|addr| {
            if let Some(Protocol::P2p(addr_peerid)) = addr.iter().last() {
                addr_peerid != peerid
            } else {
                true
            }
        })
        .collect()
}
