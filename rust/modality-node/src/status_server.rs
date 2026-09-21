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
    let status_route = warp::path::end()
        .and(warp::get())
        .and(source_filter)
        .and_then(status_handler);

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
