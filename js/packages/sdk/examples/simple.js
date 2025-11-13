/**
 * Simple example demonstrating how to use the Modal Money SDK
 * 
 * Usage:
 *   node examples/simple.js
 * 
 * Prerequisites:
 *   - A running Modal Money node with WebSocket listener
 *   - Update the MULTIADDR below with your node's multiaddr
 */

import { ModalClient, ConnectionError, TimeoutError } from '../src/index.js';

// Update this with your node's multiaddr
const MULTIADDR = '/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN';

async function main() {
  console.log('Modal Money SDK Example\n');
  
  // Create client with 10 second timeout
  const client = new ModalClient({ timeout: 10000 });
  
  try {
    // Connect to node
    console.log('Connecting to node...');
    console.log('Multiaddr:', MULTIADDR);
    await client.connect(MULTIADDR);
    console.log('✓ Connected!\n');
    
    // Get connection info
    console.log('Connected to:', client.getConnectedPeer());
    console.log('Connection status:', client.isConnected() ? 'Connected' : 'Disconnected');
    console.log();
    
    // Ping the node
    console.log('Sending ping...');
    const pingResult = await client.ping({ 
      message: 'hello from SDK',
      timestamp: Date.now() 
    });
    console.log('Ping response:', JSON.stringify(pingResult, null, 2));
    console.log();
    
    // Inspect node
    console.log('Inspecting node...');
    const inspectResult = await client.inspect({ level: 'basic' });
    console.log('Node inspection:');
    console.log('  Peer ID:', inspectResult.data.peer_id);
    console.log('  Status:', inspectResult.data.status);
    
    if (inspectResult.data.datastore) {
      const ds = inspectResult.data.datastore;
      console.log('  Datastore:');
      console.log('    Total blocks:', ds.total_blocks);
      console.log('    Block range:', ds.block_range);
      console.log('    Chain tip height:', ds.chain_tip_height);
      console.log('    Chain tip hash:', ds.chain_tip_hash);
    }
    console.log();
    
    console.log('✓ All operations completed successfully!');
    
  } catch (error) {
    console.error('\n✗ Error occurred:');
    
    if (error instanceof ConnectionError) {
      console.error('  Connection error:', error.message);
      console.error('  Peer:', error.peer);
    } else if (error instanceof TimeoutError) {
      console.error('  Timeout error:', error.message);
      console.error('  Timeout:', error.timeout, 'ms');
    } else {
      console.error('  Unexpected error:', error.message);
      console.error(error);
    }
  } finally {
    // Always clean up
    console.log('\nClosing connection...');
    await client.close();
    console.log('✓ Connection closed');
  }
}

// Run the example
main().catch(console.error);

