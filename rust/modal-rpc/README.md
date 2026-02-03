# Modal RPC

JSON-RPC interface for Modal Money hubs and networks, with WebSocket support.

## Overview

This crate provides a common RPC interface that can be used by both:
- **Contract Hubs**: Centralized contract storage and collaboration servers
- **Network Nodes**: Decentralized Modal Money network validators

## Features

- JSON-RPC 2.0 compliant
- HTTP POST endpoint for standard requests
- WebSocket support for subscriptions and real-time events
- Extensible handler trait for custom implementations

## Usage

### Implementing a Handler

```rust
use modal_rpc::{RpcHandler, types::*};
use async_trait::async_trait;

struct MyHub {
    // Your state
}

#[async_trait]
impl RpcHandler for MyHub {
    async fn get_health(&self) -> Result<HealthResponse, RpcError> {
        Ok(HealthResponse {
            status: "ok".to_string(),
            version: "0.1.0".to_string(),
            node_type: NodeType::Hub,
        })
    }

    async fn get_block_height(&self) -> Result<BlockHeightResponse, RpcError> {
        Ok(BlockHeightResponse {
            height: 12345,
            hash: Some("abc123...".to_string()),
            timestamp: Some(1234567890),
        })
    }

    async fn get_contract(&self, params: GetContractParams) -> Result<ContractResponse, RpcError> {
        // Your implementation
    }

    // ... implement other methods
}
```

### Running the Server

```rust
use modal_rpc::{RpcServer, RpcServerConfig};

#[tokio::main]
async fn main() {
    let handler = MyHub::new();
    let config = RpcServerConfig {
        host: "0.0.0.0".to_string(),
        port: 8899,
        ..Default::default()
    };

    let server = RpcServer::new(handler, config);
    server.run().await.unwrap();
}
```

### Using the Client

```rust
use modal_rpc::{RpcClient, RpcClientConfig};

#[tokio::main]
async fn main() {
    let config = RpcClientConfig {
        url: "ws://localhost:8899/ws".to_string(),
        ..Default::default()
    };

    let client = RpcClient::connect(config).await.unwrap();

    // Get health
    let health = client.get_health().await.unwrap();
    println!("Status: {}", health.status);

    // Get block height
    let height = client.get_block_height().await.unwrap();
    println!("Block height: {}", height.height);

    // Get contract
    let contract = client.get_contract("contract-id", true, true).await.unwrap();
    println!("Contract head: {:?}", contract.head);
}
```

## RPC Methods

### System Methods

| Method | Description |
|--------|-------------|
| `getHealth` | Health check |
| `getVersion` | Get API version |

### Block/Chain Methods

| Method | Description |
|--------|-------------|
| `getBlockHeight` | Get current block height |
| `getBlock` | Get block by height or hash |
| `getLatestBlockhash` | Get latest block hash |

### Contract Methods

| Method | Description |
|--------|-------------|
| `getContract` | Get contract info |
| `getContractState` | Get derived state |
| `getCommits` | Get commits for contract |
| `getCommit` | Get specific commit |
| `submitCommit` | Submit a new commit |

### Subscription Methods (WebSocket)

| Method | Description |
|--------|-------------|
| `subscribe` | Subscribe to events |
| `unsubscribe` | Unsubscribe |

### Network Methods

| Method | Description |
|--------|-------------|
| `getNetworkInfo` | Get network info |
| `getValidators` | Get validator set |
| `getEpochInfo` | Get epoch info |

## WebSocket Events

Subscribe to real-time events:

```rust
// Subscribe
let sub = client.subscribe(Some("contract-id"), vec![EventType::NewCommit]).await?;

// Handle events
let mut rx = client.take_event_receiver().unwrap();
while let Some(event) = rx.recv().await {
    match event.event_type {
        EventType::NewCommit => {
            println!("New commit: {:?}", event.data);
        }
        _ => {}
    }
}
```

## Error Codes

| Code | Description |
|------|-------------|
| -32700 | Parse error |
| -32600 | Invalid request |
| -32601 | Method not found |
| -32602 | Invalid params |
| -32603 | Internal error |
| -32000 | Contract not found |
| -32001 | Block not found |
| -32002 | Commit not found |
| -32003 | Invalid signature |
| -32004 | Rule violation |

## Example Requests

### HTTP POST

```bash
curl -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getBlockHeight",
    "params": {}
  }'
```

### WebSocket

```javascript
const ws = new WebSocket('ws://localhost:8899/ws');

ws.onopen = () => {
  ws.send(JSON.stringify({
    jsonrpc: '2.0',
    id: 1,
    method: 'getContract',
    params: { contract_id: 'abc123' }
  }));
};

ws.onmessage = (event) => {
  const response = JSON.parse(event.data);
  console.log(response);
};
```

## Integration

This crate is used by:
- `services/contract-hub/` - Contract collaboration hub
- `modal-node/` - Network node

## License

MIT
