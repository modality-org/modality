/**
 * Hub Client for Trustless Escrow Demo
 * 
 * Connects to Modality Contract Hub via WebSocket for real-time updates.
 */

export class HubClient {
  constructor(hubUrl = 'ws://localhost:3100/ws') {
    this.hubUrl = hubUrl;
    this.httpUrl = hubUrl.replace('ws://', 'http://').replace('/ws', '');
    this.ws = null;
    this.connected = false;
    this.listeners = new Map();
    this.contractId = null;
  }

  /**
   * Connect to the hub WebSocket
   */
  async connect() {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.hubUrl);
      
      this.ws.onopen = () => {
        this.connected = true;
        resolve();
      };
      
      this.ws.onerror = (err) => {
        reject(new Error('WebSocket connection failed'));
      };
      
      this.ws.onclose = () => {
        this.connected = false;
        this.emit('disconnected');
      };
      
      this.ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data);
          this.handleMessage(msg);
        } catch (err) {
          console.error('Failed to parse message:', err);
        }
      };
    });
  }

  /**
   * Handle incoming WebSocket message
   */
  handleMessage(msg) {
    switch (msg.type) {
      case 'connected':
        this.emit('connected', { version: msg.version });
        break;
        
      case 'subscribed':
        this.emit('subscribed', { contractId: msg.contract_id });
        break;
        
      case 'commit':
        this.emit('commit', {
          contractId: msg.contract_id,
          commit: msg.commit
        });
        break;
        
      case 'error':
        this.emit('error', { message: msg.message });
        break;
    }
  }

  /**
   * Subscribe to contract updates
   */
  subscribe(contractId) {
    this.contractId = contractId;
    if (this.ws && this.connected) {
      this.ws.send(JSON.stringify({
        type: 'subscribe',
        contract_id: contractId
      }));
    }
  }

  /**
   * Add event listener
   */
  on(event, callback) {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, []);
    }
    this.listeners.get(event).push(callback);
  }

  /**
   * Emit event to listeners
   */
  emit(event, data) {
    const callbacks = this.listeners.get(event) || [];
    callbacks.forEach(cb => cb(data));
  }

  /**
   * Create a new contract on the hub
   */
  async createContract(contractId, ownerId) {
    const res = await fetch(`${this.httpUrl}/contracts`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        contract_id: contractId,
        owner_id: ownerId
      })
    });
    return res.json();
  }

  /**
   * Get contract state
   */
  async getContract(contractId) {
    const res = await fetch(`${this.httpUrl}/rpc`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'getContract',
        params: { contract_id: contractId, include_commits: true }
      })
    });
    const data = await res.json();
    return data.result;
  }

  /**
   * Submit a commit
   */
  async submitCommit(contractId, commit) {
    const res = await fetch(`${this.httpUrl}/rpc`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'submitCommit',
        params: { contract_id: contractId, commit }
      })
    });
    const data = await res.json();
    return data.result;
  }

  /**
   * Close connection
   */
  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }
}

export default HubClient;
