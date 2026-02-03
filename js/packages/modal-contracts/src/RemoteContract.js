/**
 * RemoteContract - A contract synced with a hub
 */

import { Contract } from './Contract.js';
import { Commit } from './Commit.js';
import { HubClient, HubError } from './HubClient.js';

/**
 * Contract with hub synchronization
 */
export class RemoteContract extends Contract {
  /**
   * @param {object} options
   * @param {string} options.id - Contract ID
   * @param {string} options.hubUrl - Hub URL
   * @param {HubClient} options.hub - Existing HubClient (alternative to hubUrl)
   */
  constructor(options = {}) {
    super({ id: options.id });
    
    if (options.hub) {
      this.hub = options.hub;
    } else if (options.hubUrl) {
      this.hub = new HubClient(options.hubUrl);
    } else {
      this.hub = null;
    }
    
    this._remoteHead = null;
    this._unpushedCommits = [];
  }

  /**
   * Connect to a hub
   * @param {string} url
   * @returns {RemoteContract}
   */
  connect(url) {
    this.hub = new HubClient(url);
    return this;
  }

  /**
   * Check if connected to hub
   * @returns {boolean}
   */
  isConnected() {
    return this.hub !== null;
  }

  /**
   * Get number of unpushed commits
   * @returns {number}
   */
  unpushedCount() {
    return this._unpushedCommits.length;
  }

  /**
   * Pull latest commits from hub
   * @returns {Promise<{pulled: number, head: string}>}
   */
  async pull() {
    if (!this.hub) {
      throw new HubError('Not connected to hub', 'NOT_CONNECTED');
    }

    try {
      const { commits, head } = await this.hub.pull(this.id, this._remoteHead);
      
      for (const commitData of commits) {
        const commit = this._commitFromHubData(commitData);
        this.commits.push(commit);
        this._applyCommit(commit);
      }
      
      this._remoteHead = head;
      
      return { pulled: commits.length, head };
    } catch (error) {
      if (error.code === -32000) {
        // Contract not found - that's OK for new contracts
        return { pulled: 0, head: null };
      }
      throw error;
    }
  }

  /**
   * Push unpushed commits to hub
   * @returns {Promise<{pushed: number, head: string}>}
   */
  async push() {
    if (!this.hub) {
      throw new HubError('Not connected to hub', 'NOT_CONNECTED');
    }

    if (this._unpushedCommits.length === 0) {
      return { pushed: 0, head: this._remoteHead };
    }

    const toSubmit = this._unpushedCommits.map(commit => ({
      parent: commit.parent,
      body: this._commitToHubBody(commit),
      head: this._commitToHubHead(commit),
    }));

    const { pushed, head } = await this.hub.push(this.id, toSubmit);
    
    this._unpushedCommits = this._unpushedCommits.slice(pushed);
    this._remoteHead = head;
    
    return { pushed, head };
  }

  /**
   * Sync with hub (pull then push)
   * @returns {Promise<{pulled: number, pushed: number, head: string}>}
   */
  async sync() {
    const pullResult = await this.pull();
    const pushResult = await this.push();
    
    return {
      pulled: pullResult.pulled,
      pushed: pushResult.pushed,
      head: pushResult.head || pullResult.head,
    };
  }

  /**
   * Override addCommit to track unpushed
   */
  async addCommit(commit) {
    const result = await super.addCommit(commit);
    if (result.success) {
      this._unpushedCommits.push(commit);
    }
    return result;
  }

  /**
   * Convert hub commit data to Commit object
   * @private
   */
  _commitFromHubData(data) {
    const payload = data.payload || {};
    const body = payload.body || [];
    
    // Determine commit type from body
    let type = 'POST';
    let path = null;
    let commitPayload = null;
    
    if (Array.isArray(body) && body.length > 0) {
      const action = body[0];
      const method = (action.method || '').toLowerCase();
      
      switch (method) {
        case 'rule':
          type = 'RULE';
          commitPayload = action.value;
          break;
        case 'action':
          type = 'ACTION';
          path = action.action;
          commitPayload = action.params || {};
          break;
        case 'delete':
          type = 'DELETE';
          path = action.path;
          break;
        default:
          type = 'POST';
          path = action.path;
          commitPayload = action.value;
      }
    }

    return new Commit({
      type,
      parent: data.parent,
      path,
      payload: commitPayload,
      signatures: data.signatures || [],
    });
  }

  /**
   * Convert Commit to hub body format
   * @private
   */
  _commitToHubBody(commit) {
    switch (commit.type) {
      case 'RULE':
        return [{ method: 'rule', value: commit.payload }];
      case 'ACTION':
        return [{ method: 'action', action: commit.path, params: commit.payload }];
      case 'DELETE':
        return [{ method: 'delete', path: commit.path }];
      default:
        return [{ method: 'post', path: commit.path, value: commit.payload }];
    }
  }

  /**
   * Convert Commit to hub head format
   * @private
   */
  _commitToHubHead(commit) {
    const signatures = {};
    for (const sig of commit.signatures) {
      signatures[sig.publicKey] = sig.signature;
    }
    return { signatures };
  }

  /**
   * Create a RemoteContract from hub
   * @param {string} contractId
   * @param {string|HubClient} hub - Hub URL or client
   * @returns {Promise<RemoteContract>}
   */
  static async fromHub(contractId, hub) {
    const client = typeof hub === 'string' ? new HubClient(hub) : hub;
    
    const contract = new RemoteContract({
      id: contractId,
      hub: client,
    });
    
    await contract.init();
    await contract.pull();
    
    return contract;
  }
}

export default RemoteContract;
