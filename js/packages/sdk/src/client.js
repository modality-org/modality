import { createLibp2p } from 'libp2p';
import { webSockets } from '@libp2p/websockets';
import { noise } from '@chainsafe/libp2p-noise';
import { yamux } from '@chainsafe/libp2p-yamux';
import { identify } from '@libp2p/identify';
import { ReqResClient } from './reqres-client.js';
import { parseMultiaddr, extractPeerId, isValidConnectionAddr } from './utils/multiaddr.js';
import { ConnectionError, NodeError, TimeoutError } from './utils/errors.js';

/**
 * Modal Money SDK Client
 * 
 * Lightweight libp2p client for connecting to Modal Money observer nodes.
 * Operates in client-only mode by default - can dial out but cannot be dialed back.
 */
export class ModalClient {
  constructor(options = {}) {
    this.options = {
      timeout: 30000,
      clientOnly: true, // Default to client-only mode for browser compatibility
      ...options,
    };
    
    // Validate: if clientOnly is true, addresses must not be provided
    if (this.options.clientOnly && this.options.addresses) {
      throw new Error('Cannot specify addresses in clientOnly mode. Set clientOnly: false to allow listening.');
    }
    
    this.libp2p = null;
    this.reqres = null;
    this.connectedPeer = null;
  }

  /**
   * Initialize the libp2p client
   * @private
   */
  async _initLibp2p() {
    if (this.libp2p) {
      return;
    }

    const config = {
      transports: [webSockets()],
      connectionEncrypters: [noise()],
      streamMuxers: [yamux()],
      connectionManager: {
        minConnections: 0,
        maxConnections: this.options.clientOnly ? 10 : 100, // Limit for clients
        inboundConnectionThreshold: Infinity, // No inbound limit (we won't have any in client mode)
      },
      start: false,
    };

    // Explicitly configure for client-only mode
    if (this.options.clientOnly) {
      config.addresses = {
        listen: [], // Explicitly no listeners
        announce: [], // Don't announce any addresses
      };
      
      // Configure identify service to not advertise addresses
      config.services = {
        identify: identify({
          // Basic identify only - won't advertise observed addresses
        }),
      };
    }

    this.libp2p = await createLibp2p(config);

    this.reqres = new ReqResClient(this.libp2p, {
      timeout: this.options.timeout,
    });

    await this.libp2p.start();
  }

  /**
   * Connect to a Modal Money node
   * @param {string} multiaddr - Node multiaddr (e.g., '/ip4/127.0.0.1/tcp/10001/ws/p2p/12D3...')
   * @returns {Promise<void>}
   * @throws {ConnectionError} If connection fails
   */
  async connect(multiaddr) {
    try {
      // Validate multiaddr
      const ma = parseMultiaddr(multiaddr);
      if (!isValidConnectionAddr(ma)) {
        throw new ConnectionError(
          'Multiaddr must include transport and peer ID (e.g., /ip4/127.0.0.1/tcp/10001/ws/p2p/12D3...)',
          multiaddr
        );
      }

      // Initialize libp2p if needed
      await this._initLibp2p();

      // Extract peer ID
      const peerId = extractPeerId(ma);
      if (!peerId) {
        throw new ConnectionError('Could not extract peer ID from multiaddr', multiaddr);
      }

      // Dial the peer
      await this.libp2p.dial(ma);
      this.connectedPeer = ma;

      // Verify protocol support
      const supported = await this.reqres.isProtocolSupported(ma);
      if (!supported) {
        throw new ConnectionError(
          'Peer does not support Modal Money reqres protocol',
          multiaddr
        );
      }
      
      // Verify we're still client-only after connecting
      if (this.options.clientOnly && !this.isClientOnly()) {
        throw new ConnectionError(
          'Node unexpectedly started listening. This should not happen in client-only mode.',
          multiaddr
        );
      }
    } catch (error) {
      if (error instanceof ConnectionError) {
        throw error;
      }
      throw new ConnectionError(`Failed to connect to ${multiaddr}: ${error.message}`, multiaddr);
    }
  }

  /**
   * Check if client is connected to a node
   * @returns {boolean} True if connected
   */
  isConnected() {
    return this.connectedPeer !== null && this.libp2p !== null;
  }

  /**
   * Get the currently connected peer multiaddr
   * @returns {string|null} Connected peer multiaddr or null
   */
  getConnectedPeer() {
    return this.connectedPeer ? this.connectedPeer.toString() : null;
  }

  /**
   * Ping the connected node
   * @param {*} data - Data to send (will be echoed back)
   * @returns {Promise<Object>} Response object { ok, data, errors }
   * @throws {Error} If not connected or request fails
   */
  async ping(data = {}) {
    this._ensureConnected();

    try {
      const response = await this.reqres.call(this.connectedPeer, '/ping', data);
      
      if (!response.ok) {
        throw new NodeError('Node returned error for ping', response.errors);
      }

      return response;
    } catch (error) {
      if (error instanceof TimeoutError) {
        throw new TimeoutError(`Ping timeout after ${this.options.timeout}ms`, this.options.timeout);
      }
      throw error;
    }
  }

  /**
   * Inspect the connected node
   * @param {Object} options - Inspection options
   * @param {string} options.level - Inspection level ('basic', 'detailed', etc.)
   * @returns {Promise<Object>} Response object { ok, data, errors }
   * @throws {Error} If not connected or request fails
   */
  async inspect(options = {}) {
    this._ensureConnected();

    const requestData = {
      level: options.level || 'basic',
      ...options,
    };

    try {
      const response = await this.reqres.call(this.connectedPeer, '/inspect', requestData);
      
      if (!response.ok) {
        throw new NodeError('Node returned error for inspect', response.errors);
      }

      return response;
    } catch (error) {
      if (error instanceof TimeoutError) {
        throw new TimeoutError(`Inspect timeout after ${this.options.timeout}ms`, this.options.timeout);
      }
      throw error;
    }
  }

  /**
   * Make a raw request to the connected node
   * @param {string} path - Request path
   * @param {*} data - Request data
   * @returns {Promise<Object>} Response object { ok, data, errors }
   */
  async request(path, data = {}) {
    this._ensureConnected();
    return await this.reqres.call(this.connectedPeer, path, data);
  }

  /**
   * Ensure client is connected
   * @private
   * @throws {Error} If not connected
   */
  _ensureConnected() {
    if (!this.isConnected()) {
      throw new Error('Not connected. Call connect() first.');
    }
  }

  /**
   * Close the connection and cleanup
   * @returns {Promise<void>}
   */
  async close() {
    if (this.libp2p) {
      await this.libp2p.stop();
      this.libp2p = null;
      this.reqres = null;
      this.connectedPeer = null;
    }
  }

  /**
   * Get libp2p instance (for advanced usage)
   * @returns {Libp2p|null} The underlying libp2p instance
   */
  getLibp2p() {
    return this.libp2p;
  }

  /**
   * Verify that the node is in client-only mode and not listening
   * @returns {boolean} True if definitely client-only
   */
  isClientOnly() {
    if (!this.libp2p) {
      return this.options.clientOnly;
    }
    
    // Check that libp2p has no listeners
    const multiaddrs = this.libp2p.getMultiaddrs();
    return multiaddrs.length === 0;
  }

  /**
   * Get diagnostic information about client mode
   * @returns {Object} Diagnostic info
   */
  getClientModeDiagnostics() {
    return {
      clientOnly: this.options.clientOnly,
      hasListeners: this.libp2p ? this.libp2p.getMultiaddrs().length > 0 : false,
      multiaddrs: this.libp2p ? this.libp2p.getMultiaddrs().map(ma => ma.toString()) : [],
      connections: this.libp2p ? this.libp2p.getConnections().length : 0,
    };
  }
}

/**
 * Create a client-only configuration with safe defaults
 * @param {Object} options - Additional options
 * @returns {Object} Configuration object
 */
export function createClientOnlyConfig(options = {}) {
  return {
    clientOnly: true,
    timeout: options.timeout || 30000,
    // Validation: reject any listener-related options
    addresses: undefined,
    listeners: undefined,
    ...options,
  };
}

