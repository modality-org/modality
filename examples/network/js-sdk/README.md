# JavaScript SDK with devnet1 Example

This example demonstrates how to connect to a Modal Money network using the JavaScript SDK. It shows a complete workflow of starting a validator node and interacting with it from JavaScript.

## What This Example Demonstrates

1. **Starting a devnet1 validator node** using the Rust CLI
2. **Connecting to the node** from JavaScript using the SDK
3. **Pinging the node** to verify connectivity  
4. **Inspecting node state** to get chain and network information
5. **Client-only mode** - SDK cannot be dialed back (secure for browsers)

## Prerequisites

- Modal CLI built and in PATH (`modal` command available)
- Node.js 18+ installed
- pnpm installed (or npm)
- Dependencies installed in `js/` directory

## Quick Start

### 1. Install JavaScript Dependencies

From the monorepo root:

```bash
cd js
pnpm install
```

### 2. Start devnet1 Node

```bash
./01-start-devnet1.sh
```

This will:
- Create a validator node with devnet1/node1 identity
- Start it in the background
- Node listens on `/ip4/0.0.0.0/tcp/10101/ws`
- Peer ID: `12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd`

### 3. Connect with JavaScript SDK

```bash
node 03-connect-sdk.js
```

This will:
- Create a ModalClient instance
- Connect to the running node via WebSocket
- Ping the node and measure response time
- Inspect the node to get network state
- Display chain information (blocks, height, etc.)
- Demonstrate client-only mode (no inbound connections)

### 4. Stop the Node

```bash
./02-stop-devnet1.sh
```

## Example Output

```
======================================================================
Modal Money JavaScript SDK - devnet1 Connection Example
======================================================================

1. Creating SDK client...
   ✓ Client created
   ✓ Client-only mode: true

2. Connecting to devnet1 node1...
   Multiaddr: /ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW...
   ✓ Connected!

3. Verifying client-only mode...
   Client-only: true
   Has listeners: false
   Advertised addresses: 0
   Active connections: 1
   ✓ Client-only mode verified (cannot be dialed back)

4. Pinging node...
   ✓ Ping successful
   Response OK: true
   Round-trip time: 15 ms
   Echo data: { source: 'js-sdk-example', timestamp: 1699876543210 }

5. Inspecting node state...
   ✓ Inspection successful

   Node Information:
   --------------------------------------------------
   Peer ID: 12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd
   Status: running

   Datastore (Chain State):
   --------------------------------------------------
   Total blocks: 0
   Block range: None (empty chain)
   Chain tip: No blocks yet

6. Connection Summary:
   --------------------------------------------------
   ✓ Successfully connected to devnet1
   ✓ Node is responsive (ping: 15ms)
   ✓ Network state retrieved
   ✓ Client operating in secure mode (no inbound connections)

7. Closing connection...
   ✓ Connection closed

======================================================================
Example completed successfully!
======================================================================
```

## What's Happening

### Network Setup

The example uses **devnet1**, which is a development network configuration:
- Single validator (node1)
- Pre-configured identity and genesis block
- WebSocket transport for browser/JavaScript compatibility
- Port 10101 for P2P communication

### SDK Connection

The JavaScript SDK:
- Creates a lightweight libp2p client (client-only mode)
- Connects via WebSocket to the validator
- Uses the Modal Money reqres protocol for communication
- Can ping and inspect, but cannot be dialed back

### Client-Only Mode

The SDK operates in client-only mode, which means:
- ✅ Can dial out to connect to nodes
- ❌ Cannot be dialed back (no listening)
- ✅ Works behind NAT/firewalls
- ✅ Safe for browsers (browser security model)
- ✅ No port forwarding needed

## Files

- `01-start-devnet1.sh` - Start devnet1 validator node
- `02-stop-devnet1.sh` - Stop the validator node
- `03-connect-sdk.js` - JavaScript SDK example code
- `test.sh` - Automated test script
- `tmp/` - Temporary directory for node data (gitignored)

## Troubleshooting

### Node won't start

```bash
# Check if port 10101 is already in use
netstat -an | grep 10101

# Check node logs
cat ./tmp/node1-output.log

# Try manual start
modal node run-validator --dir ./tmp/node1
```

### SDK can't connect

```bash
# Verify node is running
ps aux | grep modal

# Check if node is listening
netstat -an | grep 10101

# Try manual inspection
modal node inspect --dir ./tmp/node1

# Check WebSocket is enabled in config
cat ./tmp/node1/config.json
```

### Connection timeouts

- Increase timeout in code: `new ModalClient({ timeout: 30000 })`
- Check firewall settings
- Verify node is fully initialized (wait a few seconds after start)

## Integration with Other Examples

This example can be combined with:
- **05-mining** - Mine some blocks, then inspect them via SDK
- **06-contract-lifecycle** - Deploy contracts via CLI, query via SDK  
- **07-contract-assets** - Create assets, check balances via SDK

## Next Steps

- Try connecting to devnet3 (3-node network)
- Add mining and inspect the growing chain
- Query specific blocks or contracts
- Build a web dashboard using the browser example
- Implement real-time updates with WebSocket subscriptions

## Learn More

- [JavaScript SDK Documentation](../../../js/packages/sdk/README.md)
- [Client-Only Mode](../../../js/packages/sdk/README.md#client-only-mode)
- [Network Examples](../README.md)
- [Modal Money Network Guide](../../../docs/network.md)

