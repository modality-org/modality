/**
 * Contract - A modal contract (append-only log of signed commits)
 */

import { Commit, CommitType } from './Commit.js';
import { PathValue, inferPathType } from './PathValue.js';
import * as wasm from '@modality-dev/wasm';

/**
 * Modal Contract - append-only log with rule verification
 */
export class Contract {
  /**
   * @param {object} options
   * @param {string} options.id - Contract identifier
   * @param {Commit[]} options.commits - Array of commits
   */
  constructor({ id, commits = [] } = {}) {
    this.id = id || crypto.randomUUID();
    this.commits = commits;
    this._state = new Map();
    this._rules = [];
    this._model = null;
    this._head = null;
    
    // Replay commits to build state
    for (const commit of commits) {
      this._applyCommit(commit);
    }
  }

  /**
   * Initialize WASM (must be called before verification)
   * @returns {Promise<Contract>}
   */
  async init() {
    await wasm.init();
    return this;
  }

  /**
   * Get the current HEAD commit hash
   * @returns {string|null}
   */
  get head() {
    return this._head;
  }

  /**
   * Get a value at a path
   * @param {string} path
   * @returns {any}
   */
  get(path) {
    return this._state.get(path);
  }

  /**
   * Check if a path exists
   * @param {string} path
   * @returns {boolean}
   */
  has(path) {
    return this._state.has(path);
  }

  /**
   * Get all paths
   * @returns {string[]}
   */
  paths() {
    return Array.from(this._state.keys());
  }

  /**
   * Get the current state as an object
   * @returns {object}
   */
  state() {
    const result = {};
    for (const [path, value] of this._state) {
      result[path] = value;
    }
    return result;
  }

  /**
   * Get the current model (if any)
   * @returns {object|null}
   */
  model() {
    return this._model;
  }

  /**
   * Get all rules
   * @returns {object[]}
   */
  rules() {
    return [...this._rules];
  }

  /**
   * Apply a commit internally (update state)
   * @param {Commit} commit
   * @private
   */
  _applyCommit(commit) {
    switch (commit.type) {
      case CommitType.POST:
        this._state.set(commit.path, commit.payload);
        // Check if it's a model
        if (commit.path.endsWith('.modality')) {
          try {
            this._model = wasm.parseModel(commit.payload);
          } catch (e) {
            // Not a valid model, ignore
          }
        }
        break;
        
      case CommitType.RULE:
        this._rules.push({
          content: commit.payload,
          commitHash: commit.hash(),
        });
        break;
        
      case CommitType.DELETE:
        this._state.delete(commit.path);
        break;
        
      case CommitType.ACTION:
        // Actions don't change state directly, but could
        // trigger model transitions if we have a model
        break;
    }
    
    this._head = commit.hash();
  }

  /**
   * Verify a commit against all rules
   * @param {Commit} commit
   * @returns {Promise<{valid: boolean, errors: string[]}>}
   */
  async verify(commit) {
    const errors = [];
    
    // For now, just check that it has signatures
    if (commit.signatures.length === 0) {
      errors.push('Commit has no signatures');
    }
    
    // TODO: Check commit against model and formulas using WASM
    // This would involve:
    // 1. Parsing rules
    // 2. Checking formulas against model state
    // 3. Verifying signature requirements
    
    return {
      valid: errors.length === 0,
      errors,
    };
  }

  /**
   * Add a commit to the contract
   * @param {Commit} commit
   * @returns {Promise<{success: boolean, errors: string[]}>}
   */
  async addCommit(commit) {
    // Verify parent
    if (commit.parent !== this._head) {
      return {
        success: false,
        errors: [`Invalid parent: expected ${this._head}, got ${commit.parent}`],
      };
    }
    
    // Verify commit
    const verification = await this.verify(commit);
    if (!verification.valid) {
      return {
        success: false,
        errors: verification.errors,
      };
    }
    
    // Apply commit
    this.commits.push(commit);
    this._applyCommit(commit);
    
    return { success: true, errors: [] };
  }

  /**
   * Create and add a POST commit
   * @param {string} path
   * @param {any} value
   * @param {Identity} signer
   * @returns {Promise<Commit>}
   */
  async post(path, value, signer) {
    const commit = Commit.post(this._head, path, value);
    await commit.sign(signer);
    await this.addCommit(commit);
    return commit;
  }

  /**
   * Create and add a RULE commit
   * @param {string} ruleContent
   * @param {Identity} signer
   * @returns {Promise<Commit>}
   */
  async addRule(ruleContent, signer) {
    const commit = Commit.rule(this._head, ruleContent);
    await commit.sign(signer);
    await this.addCommit(commit);
    return commit;
  }

  /**
   * Create and add an ACTION commit
   * @param {string} action
   * @param {object} params
   * @param {Identity} signer
   * @returns {Promise<Commit>}
   */
  async doAction(action, params, signer) {
    const commit = Commit.action(this._head, action, params);
    await commit.sign(signer);
    await this.addCommit(commit);
    return commit;
  }

  /**
   * Generate Mermaid diagram of the model (if present)
   * @returns {string|null}
   */
  mermaid() {
    if (!this._model) return null;
    return wasm.generateMermaid(this._model);
  }

  /**
   * Check a formula against the model
   * @param {string} formulaContent
   * @returns {object|null}
   */
  checkFormula(formulaContent) {
    if (!this._model) return null;
    const formulas = wasm.parseFormulas(formulaContent);
    if (formulas.length === 0) return null;
    return wasm.checkFormula(this._model, formulas[0]);
  }

  /**
   * Export contract to JSON
   * @returns {object}
   */
  toJSON() {
    return {
      id: this.id,
      commits: this.commits.map(c => c.toJSON()),
      head: this._head,
    };
  }

  /**
   * Import contract from JSON
   * @param {object} json
   * @returns {Contract}
   */
  static fromJSON(json) {
    const commits = json.commits.map(c => Commit.fromJSON(c));
    return new Contract({ id: json.id, commits });
  }

  /**
   * Create an empty contract
   * @returns {Contract}
   */
  static create() {
    return new Contract();
  }
}

export default Contract;
