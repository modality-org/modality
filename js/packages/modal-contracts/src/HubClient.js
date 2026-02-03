/**
 * HubClient - JSON-RPC client for Modal contract hubs
 * 
 * Connects to a hub server for contract storage, validation, and collaboration.
 */

/**
 * JSON-RPC client for Modal hubs
 */
export class HubClient {
  /**
   * @param {string} url - Hub URL (e.g., 'http://localhost:3000')
   * @param {object} options
   * @param {number} options.timeout - Request timeout in ms (default: 30000)
   */
  constructor(url, options = {}) {
    this.url = url.replace(/\/$/, '');
    this.timeout = options.timeout || 30000;
    this._requestId = 0;
  }

  /**
   * Make a JSON-RPC request
   * @param {string} method - RPC method name
   * @param {object} params - Method parameters
   * @returns {Promise<any>}
   */
  async _rpc(method, params = {}) {
    const id = ++this._requestId;
    
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout);

    try {
      const response = await fetch(this.url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id,
          method,
          params,
        }),
        signal: controller.signal,
      });

      if (!response.ok) {
        throw new HubError(`HTTP ${response.status}: ${response.statusText}`, 'HTTP_ERROR');
      }

      const json = await response.json();

      if (json.error) {
        throw new HubError(
          json.error.message || 'RPC error',
          json.error.code || 'RPC_ERROR',
          json.error.data
        );
      }

      return json.result;
    } catch (error) {
      if (error.name === 'AbortError') {
        throw new HubError('Request timeout', 'TIMEOUT');
      }
      if (error instanceof HubError) throw error;
      throw new HubError(error.message, 'NETWORK_ERROR');
    } finally {
      clearTimeout(timeoutId);
    }
  }

  /**
   * Check hub health
   * @returns {Promise<{status: string, version: string}>}
   */
  async health() {
    return this._rpc('get_health');
  }

  /**
   * Get hub version
   * @returns {Promise<string>}
   */
  async version() {
    return this._rpc('get_version');
  }

  /**
   * Get contract info
   * @param {string} contractId
   * @param {object} options
   * @param {boolean} options.includeCommits - Include commit list
   * @param {boolean} options.includeState - Include current state
   * @returns {Promise<ContractInfo>}
   */
  async getContract(contractId, options = {}) {
    return this._rpc('get_contract', {
      contract_id: contractId,
      include_commits: options.includeCommits || false,
      include_state: options.includeState || false,
    });
  }

  /**
   * Get contract state
   * @param {string} contractId
   * @returns {Promise<object>}
   */
  async getState(contractId) {
    return this._rpc('get_contract_state', { contract_id: contractId });
  }

  /**
   * Get commits for a contract
   * @param {string} contractId
   * @param {object} options
   * @param {number} options.limit - Max commits to return
   * @param {string} options.after - Start after this commit hash
   * @returns {Promise<{commits: CommitInfo[], hasMore: boolean}>}
   */
  async getCommits(contractId, options = {}) {
    return this._rpc('get_commits', {
      contract_id: contractId,
      limit: options.limit,
      after: options.after,
    });
  }

  /**
   * Get a specific commit
   * @param {string} contractId
   * @param {string} hash
   * @returns {Promise<CommitDetail>}
   */
  async getCommit(contractId, hash) {
    return this._rpc('get_commit', {
      contract_id: contractId,
      hash,
    });
  }

  /**
   * Submit a commit to the hub
   * @param {string} contractId
   * @param {object} commit - Commit object with body, head, parent
   * @returns {Promise<{success: boolean, hash: string, error?: string}>}
   */
  async submitCommit(contractId, commit) {
    return this._rpc('submit_commit', {
      contract_id: contractId,
      commit: {
        parent: commit.parent || null,
        payload: {
          body: commit.body || [],
          head: commit.head || {},
        },
      },
    });
  }

  /**
   * Push multiple commits to the hub
   * @param {string} contractId
   * @param {object[]} commits - Array of commits
   * @returns {Promise<{pushed: number, head: string}>}
   */
  async push(contractId, commits) {
    let pushed = 0;
    let head = null;

    for (const commit of commits) {
      const result = await this.submitCommit(contractId, commit);
      if (!result.success) {
        throw new HubError(result.error || 'Submit failed', 'SUBMIT_ERROR');
      }
      pushed++;
      head = result.hash;
    }

    return { pushed, head };
  }

  /**
   * Pull commits from the hub
   * @param {string} contractId
   * @param {string} since - Pull commits after this hash (null for all)
   * @returns {Promise<{commits: object[], head: string}>}
   */
  async pull(contractId, since = null) {
    const response = await this.getCommits(contractId, { limit: 1000 });
    
    let commits = response.commits || [];
    
    if (since) {
      const sinceIndex = commits.findIndex(c => c.hash === since);
      if (sinceIndex >= 0) {
        commits = commits.slice(sinceIndex + 1);
      }
    }

    const head = commits.length > 0 
      ? commits[commits.length - 1].hash 
      : since;

    return { commits, head };
  }
}

/**
 * Hub error with code
 */
export class HubError extends Error {
  constructor(message, code, data) {
    super(message);
    this.name = 'HubError';
    this.code = code;
    this.data = data;
  }
}

export default HubClient;
