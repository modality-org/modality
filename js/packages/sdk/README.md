# @modalmoney/sdk

Modal Money JavaScript SDK for connecting to observer nodes from web applications and Node.js.

This SDK provides a lightweight libp2p client for interacting with Modal Money nodes via the reqres protocol, supporting both browser and Node.js environments.

## Features

- 🌐 **Universal**: Works in both Node.js and modern browsers
- 🔌 **WebSocket-based**: Uses WebSocket transport for broad compatibility
- 🔒 **Secure**: Encrypted connections using the Noise protocol
- 🚀 **Lightweight**: Minimal dependencies, client-only libp2p setup
- 📡 **Direct P2P**: Connects directly to nodes via their multiaddrs

## Installation

```bash
npm install @modalmoney/sdk
```

or with pnpm:

```bash
pnpm add @modalmoney/sdk
```

## Quick Start

### Node.js

```javascript
import { ModalClient } from '@modalmoney/sdk';

async function main() {
  // Create client
  const client = new ModalClient();
  
  // Connect to a node (requires WebSocket listener)
  await client.connect('/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooW...');
  
  // Ping the node
  const pingResult = await client.ping({ message: 'hello' });
  console.log('Ping response:', pingResult);
  // { ok: true, data: { message: 'hello' }, errors: null }
  
  // Inspect node information
  const nodeInfo = await client.inspect({ level: 'basic' });
  console.log('Node peer ID:', nodeInfo.data.peer_id);
  console.log('Node status:', nodeInfo.data.status);
  console.log('Chain info:', nodeInfo.data.datastore);
  
  // Clean up
  await client.close();
}

main().catch(console.error);
```

### Browser (ES Module)

```html
<!DOCTYPE html>
<html>
<head>
  <title>Modal Money SDK Demo</title>
  <script type="module">
    import { ModalClient } from './node_modules/@modalmoney/sdk/src/index.js';
    
    async function connectToNode() {
      const client = new ModalClient();
      
      try {
        // Connect to node
        await client.connect('/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooW...');
        console.log('Connected!');
        
        // Ping
        const result = await client.ping({ timestamp: Date.now() });
        console.log('Ping result:', result);
        
        // Inspect
        const info = await client.inspect();
        console.log('Node info:', info.data);
        
        await client.close();
      } catch (error) {
        console.error('Error:', error);
      }
    }
    
    // Run on page load
    connectToNode();
  </script>
</head>
<body>
  <h1>Modal Money SDK Demo</h1>
  <p>Check the browser console for output</p>
</body>
</html>
```

### Browser (with bundler)

If using a bundler like Vite, Webpack, or Parcel:

```javascript
import { ModalClient } from '@modalmoney/sdk';

const client = new ModalClient();

// Connect on button click
document.getElementById('connect-btn').addEventListener('click', async () => {
  try {
    await client.connect('/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooW...');
    console.log('Connected to node');
  } catch (error) {
    console.error('Connection failed:', error);
  }
});

// Ping on button click
document.getElementById('ping-btn').addEventListener('click', async () => {
  const result = await client.ping({ timestamp: Date.now() });
  console.log('Ping:', result);
});
```

## API Reference

### `ModalClient`

Main SDK client for connecting to Modal Money nodes.

#### Constructor

```javascript
const client = new ModalClient(options);
```

**Options:**
- `timeout` (number): Request timeout in milliseconds. Default: 30000 (30 seconds)

#### Methods

##### `connect(multiaddr)`

Connect to a Modal Money node.

```javascript
await client.connect('/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooW...');
```

**Parameters:**
- `multiaddr` (string): Node's multiaddr including peer ID

**Returns:** Promise\<void\>

**Throws:** `ConnectionError` if connection fails

##### `isConnected()`

Check if client is connected to a node.

```javascript
const connected = client.isConnected();
```

**Returns:** boolean

##### `getConnectedPeer()`

Get the currently connected peer's multiaddr.

```javascript
const peerAddr = client.getConnectedPeer();
// Returns: '/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooW...' or null
```

**Returns:** string | null

##### `ping(data)`

Ping the connected node (data is echoed back).

```javascript
const response = await client.ping({ message: 'hello' });
// Returns: { ok: true, data: { message: 'hello' }, errors: null }
```

**Parameters:**
- `data` (any): Data to send (will be echoed back). Default: {}

**Returns:** Promise\<Response\>

**Throws:** 
- `Error` if not connected
- `TimeoutError` if request times out
- `NodeError` if node returns an error

##### `inspect(options)`

Inspect node information.

```javascript
const response = await client.inspect({ level: 'basic' });
// Returns: { ok: true, data: { peer_id, status, datastore: {...} }, errors: null }
```

**Parameters:**
- `options.level` (string): Inspection level ('basic', 'detailed', etc.). Default: 'basic'

**Returns:** Promise\<Response\>

**Throws:**
- `Error` if not connected
- `TimeoutError` if request times out
- `NodeError` if node returns an error

##### `request(path, data)`

Make a raw request to any endpoint (advanced usage).

```javascript
const response = await client.request('/custom/endpoint', { param: 'value' });
```

**Parameters:**
- `path` (string): Request path
- `data` (any): Request data. Default: {}

**Returns:** Promise\<Response\>

##### `close()`

Close the connection and cleanup resources.

```javascript
await client.close();
```

**Returns:** Promise\<void\>

##### `getLibp2p()`

Get the underlying libp2p instance for advanced usage.

```javascript
const libp2p = client.getLibp2p();
```

**Returns:** Libp2p | null

### Response Format

All methods return responses in this format:

```javascript
{
  ok: boolean,        // true if successful
  data: any,          // Response data
  errors: any | null  // Error details if ok is false
}
```

### Error Classes

```javascript
import {
  ConnectionError,
  TimeoutError,
  ProtocolError,
  NodeError,
  SDKError
} from '@modalmoney/sdk';
```

- **`SDKError`**: Base error class for all SDK errors
- **`ConnectionError`**: Connection to peer failed
- **`TimeoutError`**: Request timed out
- **`ProtocolError`**: Invalid response format
- **`NodeError`**: Node returned an error response

## Client-Only Mode

The SDK operates in **client-only mode by default**, which means:

- ✅ **Can dial out** to connect to other nodes
- ❌ **Cannot be dialed back** - no listening on any ports
- ✅ **No port forwarding needed** - works behind NAT/firewalls
- ✅ **Safe for browsers** - browsers cannot accept inbound connections anyway
- ✅ **Privacy-friendly** - your IP/address is not advertised to the network

### Why Client-Only?

Browser-based applications and many client applications cannot accept inbound connections due to:
- Browser security model (cannot listen on ports)
- Corporate firewalls and NAT
- Privacy concerns about advertising your address

The SDK is designed for this use case - you connect to nodes, but they cannot connect back to you.

### Verification

You can verify client-only mode:

```javascript
import { ModalClient } from '@modalmoney/sdk';

const client = new ModalClient();
await client.connect('/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooW...');

// Check client-only status
console.log(client.isClientOnly()); // true

// Get diagnostic info
console.log(client.getClientModeDiagnostics());
// {
//   clientOnly: true,
//   hasListeners: false,
//   multiaddrs: [],
//   connections: 1
// }

await client.close();
```

### Using the Helper Function

For explicit configuration:

```javascript
import { createClientOnlyConfig, ModalClient } from '@modalmoney/sdk';

const config = createClientOnlyConfig({ timeout: 10000 });
const client = new ModalClient(config);
// Guaranteed client-only with validation
```

### Advanced: Disabling Client-Only Mode (Not Recommended)

If you really need to allow listening (only works in Node.js, not browsers):

```javascript
// Note: This will fail in browsers
const client = new ModalClient({ 
  clientOnly: false,  // Disable client-only mode
  addresses: {
    listen: ['/ip4/0.0.0.0/tcp/0/ws']
  }
});
```

**Important:** 
- This won't work in browsers (browsers can't listen on ports)
- This is not recommended for most use cases
- You'll need to handle port forwarding and NAT traversal
- Your address will be advertised to the network

## Connection Requirements

### Node Configuration

Nodes must have WebSocket listeners enabled for SDK connections:

```json
{
  "listeners": [
    "/ip4/0.0.0.0/tcp/10001/ws"
  ]
}
```

### Multiaddr Format

The SDK requires a complete multiaddr with:
- IP address and port
- Transport protocol (tcp)
- WebSocket (`/ws`)
- Peer ID (`/p2p/12D3...`)

Example: `/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN`

## Browser Compatibility

The SDK works in modern browsers that support:
- ES modules
- WebSocket API
- Async/await

Tested on:
- Chrome 90+
- Firefox 88+
- Safari 15+
- Edge 90+

No polyfills required for modern browsers.

## Examples

### Error Handling

```javascript
import { ModalClient, ConnectionError, TimeoutError } from '@modalmoney/sdk';

const client = new ModalClient({ timeout: 10000 });

try {
  await client.connect('/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooW...');
  const result = await client.ping({ test: true });
  console.log(result);
} catch (error) {
  if (error instanceof ConnectionError) {
    console.error('Failed to connect:', error.message);
  } else if (error instanceof TimeoutError) {
    console.error('Request timed out after', error.timeout, 'ms');
  } else {
    console.error('Unexpected error:', error);
  }
} finally {
  await client.close();
}
```

### Custom Timeout

```javascript
const client = new ModalClient({ timeout: 5000 }); // 5 second timeout
```

### Checking Connection Status

```javascript
const client = new ModalClient();

if (!client.isConnected()) {
  await client.connect('/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooW...');
}

console.log('Connected to:', client.getConnectedPeer());
```

### React Example

```jsx
import { useState, useEffect } from 'react';
import { ModalClient } from '@modalmoney/sdk';

function NodeInfo() {
  const [nodeInfo, setNodeInfo] = useState(null);
  const [error, setError] = useState(null);

  useEffect(() => {
    const client = new ModalClient();

    async function fetchNodeInfo() {
      try {
        await client.connect('/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooW...');
        const info = await client.inspect();
        setNodeInfo(info.data);
      } catch (err) {
        setError(err.message);
      } finally {
        await client.close();
      }
    }

    fetchNodeInfo();
  }, []);

  if (error) return <div>Error: {error}</div>;
  if (!nodeInfo) return <div>Loading...</div>;

  return (
    <div>
      <h2>Node Info</h2>
      <p>Peer ID: {nodeInfo.peer_id}</p>
      <p>Status: {nodeInfo.status}</p>
      <p>Blocks: {nodeInfo.datastore?.total_blocks}</p>
    </div>
  );
}
```

## Development

This package is part of the Modal Money monorepo.

### Install Dependencies

```bash
pnpm install
```

### Run Tests

```bash
pnpm test
```

Note: Most tests are integration tests that require a running Modal Money node with WebSocket listener. Unit tests will run without a node.

### Build

```bash
pnpm build
```

## Troubleshooting

### Connection Refused

- Ensure the node is running and listening on WebSocket
- Check that the multiaddr is correct
- Verify firewall settings allow WebSocket connections

### CORS Errors (Browser)

- Not applicable for this SDK (uses WebSocket, not HTTP)
- Ensure node has WebSocket listener enabled

### Timeout Errors

- Increase timeout: `new ModalClient({ timeout: 60000 })`
- Check node is responsive
- Verify network connectivity

## Future Enhancements

- Connection pooling for multiple nodes
- Automatic reconnection with exponential backoff
- Contract interaction methods
- Block querying helpers
- Event subscriptions via gossipsub
- WebRTC support for better NAT traversal

## License

MIT

## Contributing

Contributions are welcome! Please see the main repository for contribution guidelines.

