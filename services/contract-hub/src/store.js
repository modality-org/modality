/**
 * Contract storage using SQLite
 * 
 * Stores contracts, commits, and access control.
 */

import Database from 'better-sqlite3';
import { randomUUID } from 'crypto';
import { mkdirSync, existsSync } from 'fs';
import { join } from 'path';

export class ContractStore {
  constructor(dataDir = './data') {
    this.dataDir = dataDir;
    
    // Ensure data directory exists
    if (!existsSync(dataDir)) {
      mkdirSync(dataDir, { recursive: true });
    }
    
    // Initialize database
    this.db = new Database(join(dataDir, 'contracts.db'));
    this.db.pragma('journal_mode = WAL');
    
    this.initSchema();
  }
  
  initSchema() {
    this.db.exec(`
      -- Identities (long-term ownership keys)
      CREATE TABLE IF NOT EXISTS identities (
        id TEXT PRIMARY KEY,
        public_key TEXT UNIQUE NOT NULL,
        created_at INTEGER NOT NULL
      );
      
      -- Access keys (session keys for API auth, linked to identity)
      CREATE TABLE IF NOT EXISTS access (
        id TEXT PRIMARY KEY,
        identity_id TEXT NOT NULL REFERENCES identities(id),
        public_key TEXT UNIQUE NOT NULL,
        name TEXT,
        expires_at INTEGER,
        revoked INTEGER DEFAULT 0,
        created_at INTEGER NOT NULL
      );
      
      -- Contracts (owned by identities, not access keys)
      CREATE TABLE IF NOT EXISTS contracts (
        id TEXT PRIMARY KEY,
        owner TEXT NOT NULL REFERENCES identities(id),
        name TEXT,
        description TEXT,
        head TEXT,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
      );
      
      -- Contract access control (grants to identities, not access keys)
      CREATE TABLE IF NOT EXISTS contract_access (
        contract_id TEXT NOT NULL REFERENCES contracts(id),
        identity_id TEXT NOT NULL REFERENCES identities(id),
        permission TEXT NOT NULL CHECK(permission IN ('read', 'write')),
        granted_at INTEGER NOT NULL,
        PRIMARY KEY (contract_id, identity_id)
      );
      
      -- Commits
      CREATE TABLE IF NOT EXISTS commits (
        contract_id TEXT NOT NULL REFERENCES contracts(id),
        hash TEXT NOT NULL,
        parent TEXT,
        data TEXT NOT NULL,
        signature TEXT,
        created_at INTEGER NOT NULL,
        PRIMARY KEY (contract_id, hash)
      );
      
      -- Proposals (for threshold signature commits)
      CREATE TABLE IF NOT EXISTS proposals (
        id TEXT PRIMARY KEY,
        contract_id TEXT NOT NULL REFERENCES contracts(id),
        payload TEXT NOT NULL,
        threshold_required INTEGER NOT NULL,
        threshold_signers TEXT NOT NULL,
        proposed_by TEXT NOT NULL,
        proposed_at INTEGER NOT NULL,
        expires_at INTEGER,
        status TEXT NOT NULL DEFAULT 'pending',
        finalized_at INTEGER,
        finalized_commit TEXT
      );
      
      CREATE TABLE IF NOT EXISTS proposal_approvals (
        proposal_id TEXT NOT NULL REFERENCES proposals(id),
        signer TEXT NOT NULL,
        signature TEXT NOT NULL,
        approved_at INTEGER NOT NULL,
        PRIMARY KEY (proposal_id, signer)
      );
      
      -- Indexes
      CREATE INDEX IF NOT EXISTS idx_contracts_owner ON contracts(owner);
      CREATE INDEX IF NOT EXISTS idx_commits_parent ON commits(contract_id, parent);
      CREATE INDEX IF NOT EXISTS idx_proposals_contract ON proposals(contract_id, status);
    `);
  }
  
  // ============================================================================
  // IDENTITY MANAGEMENT (long-term keys)
  // ============================================================================
  
  registerIdentity(publicKey) {
    const id = 'id_' + randomUUID().replace(/-/g, '').slice(0, 16);
    const now = Date.now();
    
    try {
      this.db.prepare(`
        INSERT INTO identities (id, public_key, created_at)
        VALUES (?, ?, ?)
      `).run(id, publicKey, now);
      
      return id;
    } catch (err) {
      if (err.message.includes('UNIQUE constraint')) {
        // Public key already registered, return existing ID
        const existing = this.db.prepare(
          'SELECT id FROM identities WHERE public_key = ?'
        ).get(publicKey);
        return existing.id;
      }
      throw err;
    }
  }
  
  getIdentity(identityId) {
    return this.db.prepare(`
      SELECT id, public_key, created_at
      FROM identities WHERE id = ?
    `).get(identityId);
  }
  
  getIdentityByPublicKey(publicKey) {
    return this.db.prepare(`
      SELECT id, public_key, created_at
      FROM identities WHERE public_key = ?
    `).get(publicKey);
  }
  
  // ============================================================================
  // ACCESS KEY MANAGEMENT (session keys)
  // ============================================================================
  
  createAccessKey(identityId, accessPublicKey, { name, expiresAt } = {}) {
    const id = 'acc_' + randomUUID().replace(/-/g, '').slice(0, 16);
    const now = Date.now();
    
    // Verify identity exists
    const identity = this.getIdentity(identityId);
    if (!identity) {
      throw new Error('Identity not found');
    }
    
    try {
      this.db.prepare(`
        INSERT INTO access (id, identity_id, public_key, name, expires_at, created_at)
        VALUES (?, ?, ?, ?, ?, ?)
      `).run(id, identityId, accessPublicKey, name || null, expiresAt || null, now);
      
      return id;
    } catch (err) {
      if (err.message.includes('UNIQUE constraint')) {
        throw new Error('Access key public key already registered');
      }
      throw err;
    }
  }
  
  getAccess(accessId) {
    const access = this.db.prepare(`
      SELECT a.id, a.identity_id, a.public_key, a.name, a.expires_at, a.revoked, a.created_at,
             i.public_key as identity_public_key
      FROM access a
      JOIN identities i ON a.identity_id = i.id
      WHERE a.id = ?
    `).get(accessId);
    
    if (!access) return null;
    
    // Check if expired or revoked
    if (access.revoked) {
      return null;
    }
    if (access.expires_at && access.expires_at < Date.now()) {
      return null;
    }
    
    return access;
  }
  
  listAccessKeys(identityId) {
    return this.db.prepare(`
      SELECT id, public_key, name, expires_at, revoked, created_at
      FROM access WHERE identity_id = ? AND revoked = 0
      ORDER BY created_at DESC
    `).all(identityId);
  }
  
  revokeAccessKey(accessId, identityId) {
    const result = this.db.prepare(`
      UPDATE access SET revoked = 1 
      WHERE id = ? AND identity_id = ?
    `).run(accessId, identityId);
    return result.changes > 0;
  }
  
  // ============================================================================
  // CONTRACT MANAGEMENT
  // ============================================================================
  
  createContract(identityId, { name, description } = {}) {
    const id = 'con_' + randomUUID().replace(/-/g, '').slice(0, 16);
    const now = Date.now();
    
    // Verify identity exists
    const identity = this.getIdentity(identityId);
    if (!identity) {
      throw new Error('Identity not found');
    }
    
    this.db.prepare(`
      INSERT INTO contracts (id, owner, name, description, created_at, updated_at)
      VALUES (?, ?, ?, ?, ?, ?)
    `).run(id, identityId, name || null, description || null, now, now);
    
    return id;
  }
  
  getContract(contractId) {
    const contract = this.db.prepare(`
      SELECT id, owner, name, description, head, created_at, updated_at
      FROM contracts WHERE id = ?
    `).get(contractId);
    
    if (!contract) return null;
    
    // Get access list (identities, not access keys)
    const access = this.db.prepare(`
      SELECT identity_id, permission
      FROM contract_access WHERE contract_id = ?
    `).all(contractId);
    
    contract.readers = access.filter(a => a.permission === 'read').map(a => a.identity_id);
    contract.writers = access.filter(a => a.permission === 'write').map(a => a.identity_id);
    
    return contract;
  }
  
  listContracts(identityId) {
    return this.db.prepare(`
      SELECT id, owner, name, description, head, created_at, updated_at
      FROM contracts WHERE owner = ?
      ORDER BY updated_at DESC
    `).all(identityId);
  }
  
  grantAccess(contractId, identityId, permission) {
    const now = Date.now();
    
    // Verify identity exists
    const identity = this.getIdentity(identityId);
    if (!identity) {
      throw new Error('Identity not found');
    }
    
    this.db.prepare(`
      INSERT OR REPLACE INTO contract_access (contract_id, identity_id, permission, granted_at)
      VALUES (?, ?, ?, ?)
    `).run(contractId, identityId, permission, now);
  }
  
  // ============================================================================
  // COMMIT OPERATIONS
  // ============================================================================
  
  pushCommits(contractId, commits) {
    const now = Date.now();
    let pushed = 0;
    let head = null;
    
    const insertStmt = this.db.prepare(`
      INSERT OR IGNORE INTO commits (contract_id, hash, parent, data, signature, created_at)
      VALUES (?, ?, ?, ?, ?, ?)
    `);
    
    const updateHead = this.db.prepare(`
      UPDATE contracts SET head = ?, updated_at = ? WHERE id = ?
    `);
    
    const tx = this.db.transaction(() => {
      for (const commit of commits) {
        const { hash, data, parent, signature } = commit;
        
        if (!hash || !data) {
          throw new Error('Commit must have hash and data');
        }
        
        const result = insertStmt.run(
          contractId,
          hash,
          parent || null,
          typeof data === 'string' ? data : JSON.stringify(data),
          signature || null,
          now
        );
        
        if (result.changes > 0) {
          pushed++;
          head = hash;
        }
      }
      
      // Update head to latest commit
      if (head) {
        updateHead.run(head, now, contractId);
      }
    });
    
    tx();
    
    // Get final head
    const contract = this.getContract(contractId);
    
    return { pushed, head: contract?.head };
  }
  
  pullCommits(contractId, sinceHash = null) {
    let query = `
      SELECT hash, parent, data, signature, created_at
      FROM commits WHERE contract_id = ?
    `;
    const params = [contractId];
    
    if (sinceHash) {
      // Get commits after the given hash (by created_at)
      const sinceCommit = this.db.prepare(
        'SELECT created_at FROM commits WHERE contract_id = ? AND hash = ?'
      ).get(contractId, sinceHash);
      
      if (sinceCommit) {
        query += ' AND created_at > ?';
        params.push(sinceCommit.created_at);
      }
    }
    
    query += ' ORDER BY created_at ASC';
    
    return this.db.prepare(query).all(...params).map(row => ({
      hash: row.hash,
      parent: row.parent,
      data: this.tryParseJson(row.data),
      signature: row.signature,
      created_at: row.created_at
    }));
  }
  
  getCommit(contractId, hash) {
    const row = this.db.prepare(`
      SELECT hash, parent, data, signature, created_at
      FROM commits WHERE contract_id = ? AND hash = ?
    `).get(contractId, hash);
    
    if (!row) return null;
    
    return {
      hash: row.hash,
      parent: row.parent,
      data: this.tryParseJson(row.data),
      signature: row.signature,
      created_at: row.created_at
    };
  }
  
  tryParseJson(str) {
    try {
      return JSON.parse(str);
    } catch {
      return str;
    }
  }
  
  // ============================================================================
  // PROPOSAL OPERATIONS
  // ============================================================================
  
  createProposal(contractId, proposal) {
    const now = Date.now();
    
    this.db.prepare(`
      INSERT INTO proposals (id, contract_id, payload, threshold_required, threshold_signers,
                            proposed_by, proposed_at, expires_at, status)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'pending')
    `).run(
      proposal.id,
      contractId,
      JSON.stringify(proposal.payload),
      proposal.threshold.required,
      JSON.stringify(proposal.threshold.signers),
      proposal.proposed_by,
      proposal.proposed_at || now,
      proposal.expires_at || null
    );
    
    return proposal.id;
  }
  
  getProposal(contractId, proposalId) {
    const row = this.db.prepare(`
      SELECT * FROM proposals WHERE contract_id = ? AND id = ?
    `).get(contractId, proposalId);
    
    if (!row) return null;
    
    // Get approvals
    const approvals = this.db.prepare(`
      SELECT signer, signature, approved_at FROM proposal_approvals WHERE proposal_id = ?
    `).all(proposalId);
    
    return {
      id: row.id,
      payload: JSON.parse(row.payload),
      threshold: {
        required: row.threshold_required,
        signers: JSON.parse(row.threshold_signers)
      },
      proposed_by: row.proposed_by,
      proposed_at: row.proposed_at,
      expires_at: row.expires_at,
      status: row.status,
      finalized_at: row.finalized_at,
      finalized_commit: row.finalized_commit,
      approvals: Object.fromEntries(approvals.map(a => [a.signer, { signature: a.signature, approved_at: a.approved_at }]))
    };
  }
  
  listProposals(contractId, status = null) {
    let query = 'SELECT id, status, proposed_by, proposed_at, expires_at FROM proposals WHERE contract_id = ?';
    const params = [contractId];
    
    if (status) {
      query += ' AND status = ?';
      params.push(status);
    }
    
    query += ' ORDER BY proposed_at DESC';
    
    return this.db.prepare(query).all(...params);
  }
  
  addProposalApproval(proposalId, signer, signature) {
    const now = Date.now();
    
    this.db.prepare(`
      INSERT INTO proposal_approvals (proposal_id, signer, signature, approved_at)
      VALUES (?, ?, ?, ?)
    `).run(proposalId, signer, signature, now);
    
    // Return current approval count
    const count = this.db.prepare(`
      SELECT COUNT(*) as count FROM proposal_approvals WHERE proposal_id = ?
    `).get(proposalId);
    
    return count.count;
  }
  
  updateProposalStatus(proposalId, status, finalizedCommit = null) {
    const now = Date.now();
    
    if (status === 'finalized') {
      this.db.prepare(`
        UPDATE proposals SET status = ?, finalized_at = ?, finalized_commit = ? WHERE id = ?
      `).run(status, now, finalizedCommit, proposalId);
    } else {
      this.db.prepare(`
        UPDATE proposals SET status = ? WHERE id = ?
      `).run(status, proposalId);
    }
  }
  
  expirePendingProposals(contractId) {
    const now = Date.now();
    
    const result = this.db.prepare(`
      UPDATE proposals SET status = 'expired'
      WHERE contract_id = ? AND status = 'pending' AND expires_at IS NOT NULL AND expires_at < ?
    `).run(contractId, now);
    
    return result.changes;
  }
  
  /**
   * Get overall stats for RPC
   */
  getStats() {
    const contracts = this.db.prepare(`
      SELECT COUNT(*) as count FROM contracts
    `).get();
    
    const commits = this.db.prepare(`
      SELECT COUNT(*) as count FROM commits
    `).get();
    
    const identities = this.db.prepare(`
      SELECT COUNT(*) as count FROM identities
    `).get();
    
    return {
      totalContracts: contracts.count,
      totalCommits: commits.count,
      totalIdentities: identities.count,
    };
  }
  
  /**
   * Get derived state for a contract (replay commits)
   */
  getContractState(contractId) {
    const commits = this.pullCommits(contractId);
    if (!commits || commits.length === 0) {
      return {};
    }
    
    const state = {};
    
    for (const commit of commits) {
      const data = commit.data;
      if (!data) continue;
      
      // Handle POST commits (set value at path)
      if (data.method === 'POST' && data.path) {
        state[data.path] = data.body;
      }
      
      // Handle DELETE commits
      if (data.method === 'DELETE' && data.path) {
        delete state[data.path];
      }
      
      // Handle RULE commits (store in special key)
      if (data.method === 'RULE') {
        if (!state._rules) state._rules = [];
        state._rules.push({
          content: data.body,
          commit: commit.hash,
        });
      }
      
      // Handle ACTION commits (store in special key)
      if (data.method === 'ACTION') {
        if (!state._actions) state._actions = [];
        state._actions.push({
          action: data.action,
          params: data.params,
          commit: commit.hash,
          timestamp: commit.timestamp,
        });
      }
    }
    
    return state;
  }
  
  close() {
    this.db.close();
  }
}
