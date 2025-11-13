/**
 * Client-Only Mode Demonstration
 * 
 * This example demonstrates the client-only mode features of the Modal Money SDK.
 * It shows how to verify that the SDK is operating in client-only mode and will not
 * accept inbound connections.
 * 
 * Usage:
 *   node examples/client-only-demo.js
 * 
 * Prerequisites:
 *   - A running Modal Money node with WebSocket listener
 *   - Update the MULTIADDR below with your node's multiaddr
 */

import { ModalClient, createClientOnlyConfig } from '../src/index.js';

// Update this with your node's multiaddr
const MULTIADDR = '/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3KooWPBRNBzgceXh7Z27wGoyYYz9ggwaYg2dWiwXXe8ieyFCN';

async function main() {
  console.log('='.repeat(60));
  console.log('Modal Money SDK - Client-Only Mode Demonstration');
  console.log('='.repeat(60));
  console.log();

  // Create client using helper function
  const config = createClientOnlyConfig({ timeout: 10000 });
  console.log('1. Created client configuration:');
  console.log(JSON.stringify(config, null, 2));
  console.log();

  const client = new ModalClient(config);

  // Check client-only status before connecting
  console.log('2. Client-only status before connection:');
  console.log('   isClientOnly():', client.isClientOnly());
  console.log();

  console.log('3. Diagnostics before connection:');
  const beforeDiagnostics = client.getClientModeDiagnostics();
  console.log(JSON.stringify(beforeDiagnostics, null, 2));
  console.log();

  try {
    // Connect to node
    console.log('4. Connecting to node...');
    console.log('   Multiaddr:', MULTIADDR);
    await client.connect(MULTIADDR);
    console.log('   ✓ Connected!');
    console.log();

    // Check client-only status after connecting
    console.log('5. Client-only status after connection:');
    console.log('   isClientOnly():', client.isClientOnly());
    console.log();

    console.log('6. Diagnostics after connection:');
    const afterDiagnostics = client.getClientModeDiagnostics();
    console.log(JSON.stringify(afterDiagnostics, null, 2));
    console.log();

    // Verify no listeners
    console.log('7. Verification:');
    if (afterDiagnostics.hasListeners) {
      console.log('   ❌ UNEXPECTED: Node has listeners!');
      console.log('   Listeners:', afterDiagnostics.multiaddrs);
    } else {
      console.log('   ✓ No listeners - node cannot be dialed back');
      console.log('   ✓ Client-only mode is working correctly');
    }
    console.log();

    console.log('8. Key Points:');
    console.log('   • Can dial out: ✓ (we connected to the node)');
    console.log('   • Cannot be dialed back: ✓ (no listeners)');
    console.log('   • Works behind NAT/firewall: ✓ (no port forwarding needed)');
    console.log('   • Browser compatible: ✓ (browsers can\'t listen anyway)');
    console.log('   • Privacy-friendly: ✓ (address not advertised)');
    console.log();

    // Test a ping to confirm connectivity
    console.log('9. Testing connectivity with ping...');
    const pingResult = await client.ping({ test: 'client-only-demo' });
    console.log('   ✓ Ping successful:', pingResult.ok);
    console.log();

  } catch (error) {
    console.error('\n✗ Error occurred:');
    console.error('  Message:', error.message);
    
    if (error.peer) {
      console.error('  Peer:', error.peer);
    }
    
    console.error();
    console.error('Common issues:');
    console.error('  • Node not running');
    console.error('  • Wrong multiaddr');
    console.error('  • Node doesn\'t have WebSocket listener enabled');
    console.error('  • Network connectivity issues');
  } finally {
    // Always clean up
    console.log('10. Closing connection...');
    await client.close();
    console.log('    ✓ Connection closed');
    console.log();
  }

  console.log('='.repeat(60));
  console.log('Demo completed');
  console.log('='.repeat(60));
}

// Run the demo
main().catch(console.error);

