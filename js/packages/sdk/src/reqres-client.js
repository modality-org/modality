import { pipe } from 'it-pipe';
import { toString as uint8ArrayToString, fromString as uint8ArrayFromString } from 'uint8arrays';
import { TimeoutError, ProtocolError } from './utils/errors.js';

const PROTOCOL_PREFIX = 'modality';
const PROTOCOL_NAME = 'reqres';
const PROTOCOL_VERSION = '1.0.0';
const DEFAULT_TIMEOUT = 30000; // 30 seconds

/**
 * Simplified ReqRes service for client-only usage
 * Implements the Modal Money request-response protocol
 */
export class ReqResClient {
  constructor(libp2p, options = {}) {
    this.libp2p = libp2p;
    this.protocol = `/${options.protocolPrefix || PROTOCOL_PREFIX}/${PROTOCOL_NAME}/${PROTOCOL_VERSION}`;
    this.timeout = options.timeout || DEFAULT_TIMEOUT;
  }

  /**
   * Make a request to a peer
   * @param {string|object} peer - Peer multiaddr
   * @param {string} path - Request path (e.g., '/ping', '/inspect')
   * @param {*} data - Request data (will be JSON stringified)
   * @param {object} options - Request options
   * @returns {Promise<object>} Response object { ok, data, errors }
   */
  async call(peer, path, data, options = {}) {
    const signal = options.signal || AbortSignal.timeout(this.timeout);
    let stream;
    let onAbort = () => {};

    try {
      // Open connection to peer
      const connection = await this.libp2p.dial(peer, {
        signal,
      });

      // Create a new stream with our protocol
      stream = await connection.newStream(this.protocol, {
        signal,
      });

      // Set up timeout handler
      onAbort = () => {
        stream?.abort(new TimeoutError(`Request timeout after ${this.timeout}ms`, this.timeout));
      };
      signal.addEventListener('abort', onAbort, { once: true });

      // Prepare request
      const request = JSON.stringify({
        path,
        data,
      });

      // Send request and receive response
      const response = await pipe(
        [uint8ArrayFromString(request)],
        stream,
        async function (source) {
          const chunks = [];
          for await (const chunk of source) {
            chunks.push(uint8ArrayToString(chunk.subarray()));
          }
          return chunks.join('');
        }
      );

      // Parse response
      let parsed;
      try {
        parsed = JSON.parse(response);
      } catch (error) {
        throw new ProtocolError(`Invalid JSON response: ${error.message}`, response);
      }

      // Validate response structure
      if (typeof parsed !== 'object' || parsed === null) {
        throw new ProtocolError('Response is not an object', parsed);
      }

      if (!('ok' in parsed)) {
        throw new ProtocolError('Response missing "ok" field', parsed);
      }

      return parsed;
    } catch (err) {
      if (stream) {
        stream.abort(err);
      }
      throw err;
    } finally {
      signal.removeEventListener('abort', onAbort);
      if (stream) {
        await stream.close().catch(() => {
          // Ignore close errors
        });
      }
    }
  }

  /**
   * Check if the protocol is supported by a peer
   * @param {string|object} peer - Peer to check
   * @returns {Promise<boolean>} True if protocol is supported
   */
  async isProtocolSupported(peer) {
    try {
      const connection = await this.libp2p.dial(peer);
      const protocols = await connection.remoteProtocols();
      return protocols.includes(this.protocol);
    } catch (error) {
      return false;
    }
  }
}

