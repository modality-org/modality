/**
 * Modal Money JavaScript SDK
 * 
 * This SDK provides a simple interface for interacting with Modal Money
 * observer nodes from web applications and websites using libp2p.
 */

export const version = '0.0.0';

// Main client
export { ModalClient, createClientOnlyConfig } from './client.js';

// ReqRes client (for advanced usage)
export { ReqResClient } from './reqres-client.js';

// Utilities
export { parseMultiaddr, extractPeerId, isValidConnectionAddr } from './utils/multiaddr.js';

// Error classes
export {
  SDKError,
  ConnectionError,
  TimeoutError,
  ProtocolError,
  NodeError,
} from './utils/errors.js';

