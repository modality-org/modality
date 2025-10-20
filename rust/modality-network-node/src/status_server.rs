use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

use modality_network_datastore::NetworkDatastore;

/// Start HTTP status server on the specified port
pub async fn start_status_server(
    port: u16,
    peerid: libp2p_identity::PeerId,
    datastore: Arc<Mutex<NetworkDatastore>>,
    listeners: Vec<libp2p::Multiaddr>,
) -> Result<tokio::task::JoinHandle<()>, anyhow::Error> {
    let status_route = warp::path::end()
        .and(warp::get())
        .and(with_peerid(peerid))
        .and(with_datastore(datastore.clone()))
        .and(with_listeners(listeners.clone()))
        .and_then(status_handler);

    let routes = status_route;

    log::info!("Starting HTTP status server on http://0.0.0.0:{}", port);

    let server = warp::serve(routes).bind(([0, 0, 0, 0], port));

    let handle = tokio::spawn(async move {
        server.await;
    });

    Ok(handle)
}

fn with_peerid(
    peerid: libp2p_identity::PeerId,
) -> impl Filter<Extract = (libp2p_identity::PeerId,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || peerid)
}

fn with_datastore(
    datastore: Arc<Mutex<NetworkDatastore>>,
) -> impl Filter<Extract = (Arc<Mutex<NetworkDatastore>>,), Error = std::convert::Infallible> + Clone
{
    warp::any().map(move || datastore.clone())
}

fn with_listeners(
    listeners: Vec<libp2p::Multiaddr>,
) -> impl Filter<Extract = (Vec<libp2p::Multiaddr>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || listeners.clone())
}

async fn status_handler(
    peerid: libp2p_identity::PeerId,
    datastore: Arc<Mutex<NetworkDatastore>>,
    listeners: Vec<libp2p::Multiaddr>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let ds = datastore.lock().await;
    
    // Get node status information
    let current_round = ds.get_current_round().await.unwrap_or(0);
    
    // Get the latest round number from the datastore keys
    let latest_round = ds.find_max_int_key("/blocks/round").await.ok().flatten().unwrap_or(0);
    
    drop(ds);

    // Build HTML response
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Modality Node Status</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            max-width: 900px;
            margin: 40px auto;
            padding: 20px;
            background: #0f0f0f;
            color: #e0e0e0;
        }}
        h1 {{
            color: #4a9eff;
            border-bottom: 2px solid #4a9eff;
            padding-bottom: 10px;
        }}
        .status-card {{
            background: #1a1a1a;
            border: 1px solid #333;
            border-radius: 8px;
            padding: 20px;
            margin: 20px 0;
        }}
        .status-item {{
            display: flex;
            justify-content: space-between;
            padding: 12px 0;
            border-bottom: 1px solid #2a2a2a;
        }}
        .status-item:last-child {{
            border-bottom: none;
        }}
        .label {{
            font-weight: 600;
            color: #888;
        }}
        .value {{
            color: #e0e0e0;
            font-family: 'Courier New', monospace;
            word-break: break-all;
        }}
        .status-online {{
            color: #4ade80;
            font-weight: bold;
        }}
        .header {{
            text-align: center;
            margin-bottom: 30px;
        }}
        .listeners {{
            list-style: none;
            padding: 0;
            margin: 0;
        }}
        .listeners li {{
            padding: 4px 0;
            color: #e0e0e0;
            font-family: 'Courier New', monospace;
        }}
    </style>
    <script>
        // Auto-refresh every 10 seconds
        setTimeout(function() {{
            location.reload();
        }}, 10000);
    </script>
</head>
<body>
    <div class="header">
        <h1>🟢 Modality Network Node</h1>
        <p class="status-online">Status: ONLINE</p>
    </div>
    
    <div class="status-card">
        <h2>Node Information</h2>
        <div class="status-item">
            <span class="label">Peer ID:</span>
            <span class="value">{}</span>
        </div>
        <div class="status-item">
            <span class="label">Listeners:</span>
            <div class="value">
                <ul class="listeners">
                    {}
                </ul>
            </div>
        </div>
    </div>

    <div class="status-card">
        <h2>Blockchain Status</h2>
        <div class="status-item">
            <span class="label">Current Round:</span>
            <span class="value">{}</span>
        </div>
        <div class="status-item">
            <span class="label">Latest Block Round:</span>
            <span class="value">{}</span>
        </div>
    </div>

    <div class="status-card">
        <p style="text-align: center; color: #666; font-size: 0.9em;">
            Page auto-refreshes every 10 seconds
        </p>
    </div>
</body>
</html>"#,
        peerid,
        listeners
            .iter()
            .map(|l| format!("<li>{}</li>", l))
            .collect::<Vec<_>>()
            .join("\n                    "),
        current_round,
        latest_round,
    );

    Ok(warp::reply::html(html))
}

