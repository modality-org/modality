/**
 * ContractStore - In-memory storage for contracts
 */

import { Contract } from './Contract.js';

/**
 * Contract store for managing multiple contracts
 */
export class ContractStore {
  constructor() {
    this._contracts = new Map();
  }

  /**
   * Get a contract by ID
   * @param {string} id
   * @returns {Contract|undefined}
   */
  get(id) {
    return this._contracts.get(id);
  }

  /**
   * Check if a contract exists
   * @param {string} id
   * @returns {boolean}
   */
  has(id) {
    return this._contracts.has(id);
  }

  /**
   * Add a contract to the store
   * @param {Contract} contract
   * @returns {Contract}
   */
  add(contract) {
    this._contracts.set(contract.id, contract);
    return contract;
  }

  /**
   * Create a new contract and add it to the store
   * @param {string} [id] - Optional contract ID
   * @returns {Contract}
   */
  create(id) {
    const contract = new Contract({ id });
    return this.add(contract);
  }

  /**
   * Remove a contract from the store
   * @param {string} id
   * @returns {boolean}
   */
  remove(id) {
    return this._contracts.delete(id);
  }

  /**
   * List all contract IDs
   * @returns {string[]}
   */
  list() {
    return Array.from(this._contracts.keys());
  }

  /**
   * Get all contracts
   * @returns {Contract[]}
   */
  all() {
    return Array.from(this._contracts.values());
  }

  /**
   * Get the number of contracts
   * @returns {number}
   */
  get size() {
    return this._contracts.size;
  }

  /**
   * Clear all contracts
   */
  clear() {
    this._contracts.clear();
  }

  /**
   * Export all contracts to JSON
   * @returns {object[]}
   */
  toJSON() {
    return this.all().map(c => c.toJSON());
  }

  /**
   * Import contracts from JSON
   * @param {object[]} json
   * @returns {ContractStore}
   */
  static fromJSON(json) {
    const store = new ContractStore();
    for (const contractJson of json) {
      store.add(Contract.fromJSON(contractJson));
    }
    return store;
  }

  /**
   * Iterate over contracts
   */
  *[Symbol.iterator]() {
    yield* this._contracts.values();
  }

  /**
   * Iterate over [id, contract] pairs
   */
  *entries() {
    yield* this._contracts.entries();
  }
}

export default ContractStore;
