//! HTTP status server for node monitoring.
//!
//! This module provides a web-based status page for monitoring node health,
//! blockchain state, and mining statistics.

use std::path::PathBuf;
use warp::Filter;

use crate::constants::STATUS_PAGE_REFRESH_SECS;
use crate::status_snapshot::{collect_node_status, NodeStatusSource};
use crate::templates::render_status_from_snapshot;

/// Start HTTP status server on the specified port
pub async fn start_status_server(
    port: u16,
    source: NodeStatusSource,
) -> Result<tokio::task::JoinHandle<()>, anyhow::Error> {
    let source_filter = warp::any().map(move || source.clone());
    let html_get = warp::path::end()
        .and(warp::get())
        .and(source_filter.clone())
        .and_then(status_handler);
    let html_head = warp::path::end().and(warp::head()).map(|| {
        warp::reply::with_header(
            warp::reply::with_status(warp::reply(), warp::http::StatusCode::OK),
            "content-type",
            "text/html; charset=utf-8",
        )
    });
    let json_get = warp::path("status.json")
        .and(warp::get())
        .and(source_filter)
        .and_then(status_json_handler);
    let json_head = warp::path("status.json").and(warp::head()).map(|| {
        cors_json_reply(warp::reply::with_status(
            warp::reply(),
            warp::http::StatusCode::OK,
        ))
    });
    let json_options = warp::path("status.json").and(warp::options()).map(|| {
        cors_json_reply(warp::reply::with_status(
            warp::reply(),
            warp::http::StatusCode::NO_CONTENT,
        ))
    });
    let status_route = html_get
        .or(html_head)
        .or(json_get)
        .or(json_head)
        .or(json_options);

    log::info!("Starting HTTP status server on http://0.0.0.0:{}", port);

    let server = warp::serve(status_route).bind(([0, 0, 0, 0], port));

    let handle = tokio::spawn(async move {
        server.await;
    });

    Ok(handle)
}

/// Generate status HTML content
pub async fn generate_status_html(source: &NodeStatusSource) -> Result<String, anyhow::Error> {
    let status = collect_node_status(source).await?;
    Ok(render_status_from_snapshot(&status))
}

async fn status_handler(source: NodeStatusSource) -> Result<impl warp::Reply, warp::Rejection> {
    let html = generate_status_html(&source)
        .await
        .map_err(|_| warp::reject::not_found())?;
    Ok(warp::reply::html(html))
}

async fn status_json_handler(
    source: NodeStatusSource,
) -> Result<impl warp::Reply, warp::Rejection> {
    let status = collect_node_status(&source)
        .await
        .map_err(|_| warp::reject::not_found())?;
    let genesis_hash = status
        .genesis
        .as_ref()
        .map(|g| g.hash.clone())
        .unwrap_or_default();
    let body = serde_json::json!({
        "network": status.network_name,
        "peer_id": status.peerid,
        "role": status.role_display,
        "height": status.total_miner_blocks,
        "epoch": status.current_epoch,
        "blocks_per_epoch": status.blocks_per_epoch,
        "peers": status.connected_peers,
        "round": status.current_round,
        "difficulty": status.current_difficulty,
        "cumulative_difficulty": status.cumulative_difficulty.to_string(),
        "hybrid": status.hybrid_consensus,
        "genesis_hash": genesis_hash,
        "sequencer_nomination_epoch": status.sequencer_nomination_epoch,
        "sequencer_committee_size": status.sequencer_committee.len(),
        "named_validators": status.named_validators,
        "named_validator_count": status.named_validators.len(),
        "validator_min_stake": status.validator_min_stake,
        "dest_apply_requires_cert": status.dest_apply_requires_cert,
        "active_roles": status.active_roles,
    });
    Ok(cors_json_reply(warp::reply::json(&body)))
}

fn cors_json_reply<T: warp::Reply>(reply: T) -> warp::reply::WithHeader<warp::reply::WithHeader<T>> {
    warp::reply::with_header(
        warp::reply::with_header(reply, "access-control-allow-origin", "*"),
        "access-control-allow-methods",
        "GET, HEAD, OPTIONS",
    )
}

/// Start status HTML writer task that periodically writes HTML to a directory
pub async fn start_status_html_writer(
    dir: PathBuf,
    source: NodeStatusSource,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<tokio::task::JoinHandle<()>, anyhow::Error> {
    std::fs::create_dir_all(&dir)?;

    let handle = tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(STATUS_PAGE_REFRESH_SECS));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    match generate_status_html(&source).await {
                        Ok(html) => {
                            let index_path = dir.join("index.html");
                            if let Err(e) = tokio::fs::write(&index_path, html).await {
                                log::error!("Failed to write status HTML to {}: {}", index_path.display(), e);
                            } else {
                                log::debug!("Status HTML written to {}", index_path.display());
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to generate status HTML: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    log::info!("Status HTML writer task shutting down");
                    break;
                }
            }
        }
    });

    Ok(handle)
}
