/**
 * @modality-dev/modal-contracts
 * 
 * High-level API for creating and interacting with Modal Contracts.
 * Uses WASM bindings for parsing, verification, and model checking.
 */

export { Contract } from './Contract.js';
export { Commit, CommitType } from './Commit.js';
export { Identity } from './Identity.js';
export { ContractStore } from './ContractStore.js';
export { PathValue, PathType } from './PathValue.js';
export { HubClient, HubError } from './HubClient.js';
export { RemoteContract } from './RemoteContract.js';

// Re-export WASM utilities
export * as wasm from '@modality-dev/wasm';
