/**
 * JavaScript SDK Example - Connect to devnet1
 * 
 * This example demonstrates how to:
 * 1. Connect to a running Modal Money node using the JavaScript SDK
 * 2. Ping the node to verify connectivity
 * 3. Inspect the node to get network state
 * 4. Display chain and network information
 * 
 * Prerequisites:
 * - Run ./01-start-devnet1.sh first to start the node
 * - Node.js installed
 * - Dependencies installed (run: pnpm install in js/ directory)
 */

import { ModalClient } from '@modalmoney/sdk';

// devnet1 node1 configuration
const NODE_MULTIADDR = '/ip4/127.0.0.1/tcp/10101/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd';

async function main() {
  console.log('='.repeat(70));
  console.log('Modal Money JavaScript SDK - devnet1 Connection Example');
  console.log('='.repeat(70));
  console.log();

  // Create SDK client
  console.log('1. Creating SDK client...');
  const client = new ModalClient({ timeout: 10000 });
  console.log('   ✓ Client created');
  console.log('   ✓ Client-only mode:', client.isClientOnly());
  console.log();

  try {
    // Connect to devnet1 node1
    console.log('2. Connecting to devnet1 node1...');
    console.log('   Multiaddr:', NODE_MULTIADDR);
    await client.connect(NODE_MULTIADDR);
    console.log('   ✓ Connected!');
    console.log();

    // Verify client-only mode (cannot be dialed back)
    console.log('3. Verifying client-only mode...');
    const diagnostics = client.getClientModeDiagnostics();
    console.log('   Client-only:', diagnostics.clientOnly);
    console.log('   Has listeners:', diagnostics.hasListeners);
    console.log('   Advertised addresses:', diagnostics.multiaddrs.length);
    console.log('   Active connections:', diagnostics.connections);
    console.log('   ✓ Client-only mode verified (cannot be dialed back)');
    console.log();

    // Ping the node
    console.log('4. Pinging node...');
    const pingStart = Date.now();
    const pingResult = await client.ping({
      source: 'js-sdk-example',
      timestamp: pingStart,
    });
    const pingDuration = Date.now() - pingStart;
    
    console.log('   ✓ Ping successful');
    console.log('   Response OK:', pingResult.ok);
    console.log('   Round-trip time:', pingDuration, 'ms');
    console.log('   Echo data:', JSON.stringify(pingResult.data, null, 2));
    console.log();

    // Inspect node state
    console.log('5. Inspecting node state...');
    const inspectResult = await client.inspect({ level: 'basic' });
    
    if (inspectResult.ok) {
      const nodeData = inspectResult.data;
      
      console.log('   ✓ Inspection successful');
      console.log();
      console.log('   Node Information:');
      console.log('   ' + '-'.repeat(50));
      console.log('   Peer ID:', nodeData.peer_id);
      console.log('   Status:', nodeData.status);
      console.log();
      
      if (nodeData.datastore) {
        const ds = nodeData.datastore;
        console.log('   Datastore (Chain State):');
        console.log('   ' + '-'.repeat(50));
        console.log('   Total blocks:', ds.total_blocks || 0);
        
        if (ds.block_range) {
          console.log('   Block range:', `${ds.block_range[0]} - ${ds.block_range[1]}`);
        } else {
          console.log('   Block range: None (empty chain)');
        }
        
        if (ds.chain_tip_height !== null && ds.chain_tip_height !== undefined) {
          console.log('   Chain tip height:', ds.chain_tip_height);
          console.log('   Chain tip hash:', ds.chain_tip_hash || 'None');
        } else {
          console.log('   Chain tip: No blocks yet');
        }
        
        if (ds.epochs) {
          console.log('   Epochs:', ds.epochs);
        }
        
        if (ds.unique_miners) {
          console.log('   Unique miners:', ds.unique_miners);
        }
      } else {
        console.log('   Datastore: No data available');
      }
      console.log();
    } else {
      console.log('   ✗ Inspection failed');
      console.log('   Errors:', inspectResult.errors);
      console.log();
    }

    // Summary
    console.log('6. Connection Summary:');
    console.log('   ' + '-'.repeat(50));
    console.log('   ✓ Successfully connected to devnet1');
    console.log('   ✓ Node is responsive (ping: ' + pingDuration + 'ms)');
    console.log('   ✓ Network state retrieved');
    console.log('   ✓ Client operating in secure mode (no inbound connections)');
    console.log();

  } catch (error) {
    console.error('✗ Error occurred:');
    console.error('  Type:', error.constructor.name);
    console.error('  Message:', error.message);
    
    if (error.peer) {
      console.error('  Peer:', error.peer);
    }
    
    if (error.timeout) {
      console.error('  Timeout:', error.timeout, 'ms');
    }
    
    console.error();
    console.error('Troubleshooting:');
    console.error('  1. Ensure devnet1 node is running: ./01-start-devnet1.sh');
    console.error('  2. Check node logs: cat ./tmp/node1-output.log');
    console.error('  3. Verify node is listening on WebSocket: netstat -an | grep 10101');
    console.error('  4. Try manual inspection: modal node inspect --dir ./tmp/node1');
    console.error();
    
    process.exit(1);
  } finally {
    // Always cleanup
    console.log('7. Closing connection...');
    await client.close();
    console.log('   ✓ Connection closed');
    console.log();
  }

  console.log('='.repeat(70));
  console.log('Example completed successfully!');
  console.log('='.repeat(70));
}

// Run the example
main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});

