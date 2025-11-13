/**
 * Base error class for SDK errors
 */
export class SDKError extends Error {
  constructor(message) {
    super(message);
    this.name = this.constructor.name;
    Error.captureStackTrace(this, this.constructor);
  }
}

/**
 * Error thrown when connection to a peer fails
 */
export class ConnectionError extends SDKError {
  constructor(message, peer) {
    super(message);
    this.peer = peer;
  }
}

/**
 * Error thrown when a request times out
 */
export class TimeoutError extends SDKError {
  constructor(message, timeout) {
    super(message);
    this.timeout = timeout;
  }
}

/**
 * Error thrown when receiving an invalid response from a node
 */
export class ProtocolError extends SDKError {
  constructor(message, response) {
    super(message);
    this.response = response;
  }
}

/**
 * Error thrown when node returns an error response
 */
export class NodeError extends SDKError {
  constructor(message, errors) {
    super(message);
    this.errors = errors;
  }
}

