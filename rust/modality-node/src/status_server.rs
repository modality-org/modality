//! HTTP app for a node: miner status, with contract exploration optional.

use std::path::PathBuf;
use warp::http::StatusCode;
use warp::Filter;

use crate::constants::STATUS_PAGE_REFRESH_SECS;
use crate::explorer;
use crate::status_snapshot::{chain_report, collect_node_status, NodeStatusSource};
use crate::templates::render_status_from_snapshot;

/// Start HTTP status server on the specified port
pub async fn start_status_server(
    port: u16,
    source: NodeStatusSource,
) -> Result<tokio::task::JoinHandle<()>, anyhow::Error> {
    let source_filter = warp::any().map(move || source.clone());

    let page_head = warp::head()
        .and(
            warp::path::end()
                .or(warp::path("status").and(warp::path::end()))
                .or(warp::path!("contracts" / String).map(|_id: String| ())),
        )
        .map(|_| {
            warp::reply::with_header(
                warp::reply::with_status(warp::reply(), StatusCode::OK),
                "content-type",
                "text/html; charset=utf-8",
            )
        });

    let root_get = warp::path::end()
        .and(warp::get())
        .and(source_filter.clone())
        .and_then(status_handler);
    let contract_get = warp::path!("contracts" / String)
        .and(warp::get())
        .and(source_filter.clone())
        .and_then(contract_page);
    let status_get = warp::path("status")
        .and(warp::path::end())
        .and(warp::get())
        .and(source_filter.clone())
        .and_then(status_handler);
    let json_get = warp::path("status.json")
        .and(warp::get())
        .and(source_filter.clone())
        .and_then(status_json_handler);
    let json_head = warp::path("status.json")
        .and(warp::head())
        .map(|| cors_json_reply(warp::reply::with_status(warp::reply(), StatusCode::OK)));
    let json_options = warp::path("status.json").and(warp::options()).map(|| {
        cors_json_reply(warp::reply::with_status(
            warp::reply(),
            StatusCode::NO_CONTENT,
        ))
    });
    let chain_get = warp::path("chain.json")
        .and(warp::get())
        .and(source_filter.clone())
        .and_then(chain_json_handler);
    let chain_head = warp::path("chain.json")
        .and(warp::head())
        .map(|| cors_json_reply(warp::reply::with_status(warp::reply(), StatusCode::OK)));
    let chain_options = warp::path("chain.json").and(warp::options()).map(|| {
        cors_json_reply(warp::reply::with_status(
            warp::reply(),
            StatusCode::NO_CONTENT,
        ))
    });

    let api_all = warp::path!("api" / "contracts")
        .and(warp::get())
        .and(source_filter.clone())
        .and_then(api_contracts_handler);
    let api_replay = warp::path!("api" / "contracts" / String / "replay")
        .and(warp::get())
        .and(source_filter.clone())
        .and_then(api_replay_handler);
    let api_commits = warp::path!("api" / "contracts" / String / "commits")
        .and(warp::get())
        .and(source_filter.clone())
        .and_then(api_commits_handler);
    let api_one = warp::path!("api" / "contracts" / String)
        .and(warp::get())
        .and(source_filter)
        .and_then(api_contract_handler);
    let api_options = warp::path("api").and(warp::options()).map(|| {
        cors_json_reply(warp::reply::with_status(
            warp::reply(),
            StatusCode::NO_CONTENT,
        ))
    });

    let status_route = root_get
        .or(contract_get)
        .or(status_get)
        .or(page_head)
        .or(json_get)
        .or(json_head)
        .or(json_options)
        .or(chain_get)
        .or(chain_head)
        .or(chain_options)
        .or(api_replay)
        .or(api_commits)
        .or(api_one)
        .or(api_all)
        .or(api_options);

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

async fn contract_page(
    _id: String,
    source: NodeStatusSource,
) -> Result<impl warp::Reply, warp::Rejection> {
    status_handler(source).await
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
        "chain_tip": status.chain_tip,
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
        "peer_list": status
            .peers
            .iter()
            .map(|peer| {
                serde_json::json!({
                    "peer_id": peer.peer_id,
                    "role": peer.role,
                    "status_url": peer.status_url,
                })
            })
            .collect::<Vec<_>>(),
    });
    Ok(cors_json_reply(warp::reply::json(&body)))
}

async fn chain_json_handler(
    source: NodeStatusSource,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mgr = source.datastore.lock().await;
    let canonical = modality_datastore::models::miner::MinerBlock::find_all_canonical_multi(&mgr)
        .await
        .map_err(|_| warp::reject::not_found())?;
    let orphans = modality_datastore::models::miner::MinerBlock::find_all_orphaned_multi(&mgr)
        .await
        .unwrap_or_default();
    drop(mgr);
    let body = chain_report(
        &source.peerid.to_string(),
        &source.role,
        &canonical,
        &orphans,
    );
    Ok(cors_json_reply(warp::reply::json(&body)))
}

async fn api_contracts_handler(
    source: NodeStatusSource,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mgr = source.datastore.lock().await;
    let list = explorer::list_contracts(&mgr)
        .await
        .map_err(|_| warp::reject::not_found())?;
    Ok(cors_json_status(StatusCode::OK, list))
}

async fn api_contract_handler(
    id: String,
    source: NodeStatusSource,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mgr = source.datastore.lock().await;
    match explorer::inspect_contract(&mgr, &id)
        .await
        .map_err(|_| warp::reject::not_found())?
    {
        Some(inspect) => Ok(cors_json_status(StatusCode::OK, inspect)),
        None => Ok(cors_json_status(
            StatusCode::NOT_FOUND,
            serde_json::json!({ "error": "not found" }),
        )),
    }
}

async fn api_commits_handler(
    id: String,
    source: NodeStatusSource,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mgr = source.datastore.lock().await;
    match explorer::list_commits(&mgr, &id)
        .await
        .map_err(|_| warp::reject::not_found())?
    {
        Some(commits) => Ok(cors_json_status(StatusCode::OK, commits)),
        None => Ok(cors_json_status(
            StatusCode::NOT_FOUND,
            serde_json::json!({ "error": "not found" }),
        )),
    }
}

async fn api_replay_handler(
    id: String,
    source: NodeStatusSource,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mgr = source.datastore.lock().await;
    match explorer::replay_contract(&mgr, &id, None).await {
        Ok(Some(replay)) => Ok(cors_json_status(StatusCode::OK, replay)),
        Ok(None) => Ok(cors_json_status(
            StatusCode::NOT_FOUND,
            serde_json::json!({ "error": "not found" }),
        )),
        Err(err) => Ok(cors_json_status(
            StatusCode::CONFLICT,
            serde_json::json!({ "error": err.to_string() }),
        )),
    }
}

fn cors_json_status<T: serde::Serialize>(
    status: StatusCode,
    body: T,
) -> warp::reply::WithHeader<warp::reply::WithHeader<warp::reply::WithStatus<warp::reply::Json>>> {
    cors_json_reply(warp::reply::with_status(warp::reply::json(&body), status))
}

fn cors_json_reply<T: warp::Reply>(
    reply: T,
) -> warp::reply::WithHeader<warp::reply::WithHeader<T>> {
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
