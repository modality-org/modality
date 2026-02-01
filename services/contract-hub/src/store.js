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
      -- Access keys (authentication)
      CREATE TABLE IF NOT EXISTS access (
        id TEXT PRIMARY KEY,
        public_key TEXT UNIQUE NOT NULL,
        created_at INTEGER NOT NULL
      );
      
      -- Contracts
      CREATE TABLE IF NOT EXISTS contracts (
        id TEXT PRIMARY KEY,
        owner TEXT NOT NULL REFERENCES access(id),
        name TEXT,
        description TEXT,
        head TEXT,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
      );
      
      -- Contract access control
      CREATE TABLE IF NOT EXISTS contract_access (
        contract_id TEXT NOT NULL REFERENCES contracts(id),
        access_id TEXT NOT NULL REFERENCES access(id),
        permission TEXT NOT NULL CHECK(permission IN ('read', 'write')),
        granted_at INTEGER NOT NULL,
        PRIMARY KEY (contract_id, access_id)
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
      
      -- Indexes
      CREATE INDEX IF NOT EXISTS idx_contracts_owner ON contracts(owner);
      CREATE INDEX IF NOT EXISTS idx_commits_parent ON commits(contract_id, parent);
    `);
  }
  
  // ============================================================================
  // ACCESS MANAGEMENT
  // ============================================================================
  
  registerAccess(publicKey) {
    const id = 'acc_' + randomUUID().replace(/-/g, '').slice(0, 16);
    const now = Date.now();
    
    try {
      this.db.prepare(`
        INSERT INTO access (id, public_key, created_at)
        VALUES (?, ?, ?)
      `).run(id, publicKey, now);
      
      return id;
    } catch (err) {
      if (err.message.includes('UNIQUE constraint')) {
        // Public key already registered, return existing ID
        const existing = this.db.prepare(
          'SELECT id FROM access WHERE public_key = ?'
        ).get(publicKey);
        return existing.id;
      }
      throw err;
    }
  }
  
  getAccess(accessId) {
    return this.db.prepare(`
      SELECT id, public_key, created_at
      FROM access WHERE id = ?
    `).get(accessId);
  }
  
  // ============================================================================
  // CONTRACT MANAGEMENT
  // ============================================================================
  
  createContract(ownerId, { name, description } = {}) {
    const id = 'con_' + randomUUID().replace(/-/g, '').slice(0, 16);
    const now = Date.now();
    
    this.db.prepare(`
      INSERT INTO contracts (id, owner, name, description, created_at, updated_at)
      VALUES (?, ?, ?, ?, ?, ?)
    `).run(id, ownerId, name || null, description || null, now, now);
    
    return id;
  }
  
  getContract(contractId) {
    const contract = this.db.prepare(`
      SELECT id, owner, name, description, head, created_at, updated_at
      FROM contracts WHERE id = ?
    `).get(contractId);
    
    if (!contract) return null;
    
    // Get access list
    const access = this.db.prepare(`
      SELECT access_id, permission
      FROM contract_access WHERE contract_id = ?
    `).all(contractId);
    
    contract.readers = access.filter(a => a.permission === 'read').map(a => a.access_id);
    contract.writers = access.filter(a => a.permission === 'write').map(a => a.access_id);
    
    return contract;
  }
  
  listContracts(ownerId) {
    return this.db.prepare(`
      SELECT id, owner, name, description, head, created_at, updated_at
      FROM contracts WHERE owner = ?
      ORDER BY updated_at DESC
    `).all(ownerId);
  }
  
  grantAccess(contractId, accessId, permission) {
    const now = Date.now();
    
    this.db.prepare(`
      INSERT OR REPLACE INTO contract_access (contract_id, access_id, permission, granted_at)
      VALUES (?, ?, ?, ?)
    `).run(contractId, accessId, permission, now);
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
  
  close() {
    this.db.close();
  }
}
