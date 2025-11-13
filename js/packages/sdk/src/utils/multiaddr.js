import { multiaddr } from '@multiformats/multiaddr';

/**
 * Parse and validate a multiaddr string
 * @param {string} addr - Multiaddr string (e.g., '/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3...')
 * @returns {Multiaddr} Parsed multiaddr object
 * @throws {Error} If multiaddr is invalid
 */
export function parseMultiaddr(addr) {
  try {
    return multiaddr(addr);
  } catch (error) {
    throw new Error(`Invalid multiaddr: ${addr}. ${error.message}`);
  }
}

/**
 * Extract peer ID from a multiaddr
 * @param {string|Multiaddr} addr - Multiaddr string or object
 * @returns {string|null} Peer ID string or null if not found
 */
export function extractPeerId(addr) {
  try {
    const ma = typeof addr === 'string' ? multiaddr(addr) : addr;
    const peerIdProto = ma.protos().find(p => p.name === 'p2p');
    
    if (!peerIdProto) {
      return null;
    }
    
    // Get the p2p component value
    const tuples = ma.stringTuples();
    const p2pTuple = tuples.find(([code]) => code === 421); // 421 is the p2p protocol code
    
    return p2pTuple ? p2pTuple[1] : null;
  } catch (error) {
    return null;
  }
}

/**
 * Validate that a multiaddr has required components for connection
 * @param {string|Multiaddr} addr - Multiaddr to validate
 * @returns {boolean} True if valid for connection
 */
export function isValidConnectionAddr(addr) {
  try {
    const ma = typeof addr === 'string' ? multiaddr(addr) : addr;
    const protos = ma.protos().map(p => p.name);
    
    // Must have transport (tcp or other)
    const hasTransport = protos.includes('tcp') || protos.includes('udp');
    
    // Should have peer ID for direct connection
    const hasPeerId = protos.includes('p2p');
    
    return hasTransport && hasPeerId;
  } catch (error) {
    return false;
  }
}

