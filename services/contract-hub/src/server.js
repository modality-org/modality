/**
 * Modality Contract Hub
 * 
 * Centralized HTTP service for push/pull of Modality contracts.
 * Authentication via ed25519 keypair signatures.
 */

import express from 'express';
import { createServer } from 'http';
import { ContractStore } from './store.js';
import { AuthMiddleware } from './auth.js';
import { validateCommits } from './validate.js';

const app = express();
const PORT = process.env.PORT || 3100;

// Initialize store and auth
const store = new ContractStore(process.env.DATA_DIR || './data');
const auth = new AuthMiddleware(store);

// Middleware
app.use(express.json({ limit: '10mb' }));
app.use(express.raw({ type: 'application/octet-stream', limit: '10mb' }));

// Health check (no auth)
app.get('/health', (req, res) => {
  res.json({ status: 'ok', service: 'contract-hub', version: '0.1.0' });
});

// ============================================================================
// IDENTITY MANAGEMENT (long-term keys)
// ============================================================================

/**
 * Register a new identity
 * POST /identity/register
 * Body: { public_key: hex }
 * Returns: { identity_id, public_key }
 */
app.post('/identity/register', async (req, res) => {
  try {
    const { public_key } = req.body;
    if (!public_key || typeof public_key !== 'string') {
      return res.status(400).json({ error: 'public_key required (hex string)' });
    }
    
    // Validate hex format
    if (!/^[0-9a-fA-F]{64}$/.test(public_key)) {
      return res.status(400).json({ error: 'public_key must be 64 hex chars (32 bytes)' });
    }
    
    const identityId = store.registerIdentity(public_key.toLowerCase());
    res.json({ identity_id: identityId, public_key: public_key.toLowerCase() });
  } catch (err) {
    res.status(500).json({ error: err.message });
  }
});

/**
 * Get identity info
 * GET /identity/:identityId
 */
app.get('/identity/:identityId', (req, res) => {
  const info = store.getIdentity(req.params.identityId);
  if (!info) {
    return res.status(404).json({ error: 'Identity not found' });
  }
  res.json(info);
});

// ============================================================================
// ACCESS KEY MANAGEMENT (session keys)
// ============================================================================

/**
 * Create a new access key for an identity
 * POST /access/create
 * Body: { identity_id, access_public_key, name?, expires_at?, signature }
 * signature = identity key signs: "create_access:" + access_public_key + ":" + timestamp
 * Returns: { access_id, identity_id, public_key }
 */
app.post('/access/create', async (req, res) => {
  try {
    const { identity_id, access_public_key, name, expires_at, signature, timestamp } = req.body;
    
    if (!identity_id || !access_public_key || !signature || !timestamp) {
      return res.status(400).json({ 
        error: 'Required: identity_id, access_public_key, signature, timestamp' 
      });
    }
    
    // Validate hex format
    if (!/^[0-9a-fA-F]{64}$/.test(access_public_key)) {
      return res.status(400).json({ error: 'access_public_key must be 64 hex chars' });
    }
    
    // Check timestamp (within 5 minutes)
    const ts = parseInt(timestamp, 10);
    const now = Date.now();
    if (isNaN(ts) || Math.abs(now - ts) > 5 * 60 * 1000) {
      return res.status(400).json({ error: 'Invalid or expired timestamp' });
    }
    
    // Get identity and verify signature
    const identity = store.getIdentity(identity_id);
    if (!identity) {
      return res.status(404).json({ error: 'Identity not found' });
    }
    
    // Verify identity signed this access key creation
    const verified = await auth.verifySignature(
      identity.public_key,
      `create_access:${access_public_key}:${timestamp}`,
      signature
    );
    
    if (!verified) {
      return res.status(401).json({ error: 'Invalid signature' });
    }
    
    const accessId = store.createAccessKey(identity_id, access_public_key.toLowerCase(), {
      name,
      expiresAt: expires_at
    });
    
    res.json({ 
      access_id: accessId, 
      identity_id,
      public_key: access_public_key.toLowerCase() 
    });
  } catch (err) {
    res.status(500).json({ error: err.message });
  }
});

/**
 * List access keys for an identity
 * GET /access/list
 * Auth: Access key header (must belong to identity)
 */
app.get('/access/list', auth.verify(), (req, res) => {
  const keys = store.listAccessKeys(req.identityId);
  res.json({ access_keys: keys });
});

/**
 * Revoke an access key
 * POST /access/revoke
 * Auth: Access key header
 * Body: { access_id }
 */
app.post('/access/revoke', auth.verify(), (req, res) => {
  const { access_id } = req.body;
  if (!access_id) {
    return res.status(400).json({ error: 'access_id required' });
  }
  
  const revoked = store.revokeAccessKey(access_id, req.identityId);
  if (!revoked) {
    return res.status(404).json({ error: 'Access key not found or not owned by you' });
  }
  
  res.json({ revoked: true, access_id });
});

/**
 * Get access key info
 * GET /access/:accessId
 */
app.get('/access/:accessId', (req, res) => {
  const info = store.getAccess(req.params.accessId);
  if (!info) {
    return res.status(404).json({ error: 'Access key not found or expired' });
  }
  // Don't expose identity_public_key
  const { identity_public_key, ...safe } = info;
  res.json(safe);
});

// ============================================================================
// CONTRACT OPERATIONS (authenticated)
// ============================================================================

/**
 * Create a new contract
 * POST /contracts
 * Auth: Signature header
 * Body: { name?: string, description?: string }
 * Returns: { contract_id, owner }
 */
app.post('/contracts', auth.verify(), async (req, res) => {
  try {
    const { name, description } = req.body || {};
    const contractId = store.createContract(req.identityId, { name, description });
    res.status(201).json({ contract_id: contractId, owner: req.identityId });
  } catch (err) {
    res.status(500).json({ error: err.message });
  }
});

/**
 * List contracts owned by the authenticated user
 * GET /contracts
 * Auth: Signature header
 */
app.get('/contracts', auth.verify(), (req, res) => {
  const contracts = store.listContracts(req.identityId);
  res.json({ contracts });
});

/**
 * Get contract info
 * GET /contracts/:contractId
 * Auth: Signature header (must be owner or have read access)
 */
app.get('/contracts/:contractId', auth.verify(), (req, res) => {
  const info = store.getContract(req.params.contractId);
  if (!info) {
    return res.status(404).json({ error: 'Contract not found' });
  }
  
  // Check access (based on identity, not access key)
  if (info.owner !== req.identityId && !info.readers?.includes(req.identityId)) {
    return res.status(403).json({ error: 'Access denied' });
  }
  
  res.json(info);
});

/**
 * Push commits to a contract
 * POST /contracts/:contractId/push
 * Auth: Signature header (must be owner or have write access)
 * Body: { commits: [{ hash, data, parent?, signature? }], skipValidation?: boolean }
 * 
 * Validates commits by default:
 * - Parent chain integrity
 * - Signature verification (if signed)
 * - Hash verification
 */
app.post('/contracts/:contractId/push', auth.verify(), async (req, res) => {
  try {
    const { contractId } = req.params;
    const { commits, skipValidation } = req.body;
    
    if (!commits || !Array.isArray(commits)) {
      return res.status(400).json({ error: 'commits array required' });
    }
    
    const info = store.getContract(contractId);
    if (!info) {
      return res.status(404).json({ error: 'Contract not found' });
    }
    
    // Check write access (based on identity)
    if (info.owner !== req.identityId && !info.writers?.includes(req.identityId)) {
      return res.status(403).json({ error: 'Write access denied' });
    }
    
    // Validate commits (unless explicitly skipped by owner)
    if (!skipValidation || info.owner !== req.identityId) {
      const validation = await validateCommits(store, contractId, commits);
      if (!validation.valid) {
        return res.status(400).json({ 
          error: 'Commit validation failed',
          validation_errors: validation.errors
        });
      }
    }
    
    const result = store.pushCommits(contractId, commits);
    res.json({ pushed: result.pushed, head: result.head });
  } catch (err) {
    res.status(400).json({ error: err.message });
  }
});

/**
 * Pull commits from a contract
 * GET /contracts/:contractId/pull
 * Auth: Signature header
 * Query: ?since=<commit_hash> (optional, returns commits after this hash)
 */
app.get('/contracts/:contractId/pull', auth.verify(), (req, res) => {
  const { contractId } = req.params;
  const { since } = req.query;
  
  const info = store.getContract(contractId);
  if (!info) {
    return res.status(404).json({ error: 'Contract not found' });
  }
  
  // Check read access (based on identity)
  if (info.owner !== req.identityId && !info.readers?.includes(req.identityId)) {
    return res.status(403).json({ error: 'Read access denied' });
  }
  
  const commits = store.pullCommits(contractId, since);
  res.json({ 
    contract_id: contractId,
    head: info.head,
    commits 
  });
});

/**
 * Get specific commit
 * GET /contracts/:contractId/commits/:hash
 * Auth: Signature header
 */
app.get('/contracts/:contractId/commits/:hash', auth.verify(), (req, res) => {
  const { contractId, hash } = req.params;
  
  const info = store.getContract(contractId);
  if (!info) {
    return res.status(404).json({ error: 'Contract not found' });
  }
  
  if (info.owner !== req.accessId && !info.readers?.includes(req.accessId)) {
    return res.status(403).json({ error: 'Read access denied' });
  }
  
  const commit = store.getCommit(contractId, hash);
  if (!commit) {
    return res.status(404).json({ error: 'Commit not found' });
  }
  
  res.json(commit);
});

/**
 * Grant access to a contract (grants to an identity, not access key)
 * POST /contracts/:contractId/access
 * Auth: Signature header (must be owner)
 * Body: { identity_id, permission: 'read' | 'write' }
 */
app.post('/contracts/:contractId/access', auth.verify(), (req, res) => {
  try {
    const { contractId } = req.params;
    const { identity_id, permission } = req.body;
    
    if (!identity_id || !permission) {
      return res.status(400).json({ error: 'identity_id and permission required' });
    }
    
    if (!['read', 'write'].includes(permission)) {
      return res.status(400).json({ error: 'permission must be "read" or "write"' });
    }
    
    const info = store.getContract(contractId);
    if (!info) {
      return res.status(404).json({ error: 'Contract not found' });
    }
    
    if (info.owner !== req.identityId) {
      return res.status(403).json({ error: 'Only owner can grant access' });
    }
    
    store.grantAccess(contractId, identity_id, permission);
    res.json({ granted: true, identity_id, permission });
  } catch (err) {
    res.status(500).json({ error: err.message });
  }
});

// ============================================================================
// START SERVER
// ============================================================================

const server = createServer(app);

server.listen(PORT, () => {
  console.log(`🔐 Contract Hub running on http://localhost:${PORT}`);
  console.log(`   Data directory: ${store.dataDir}`);
});

export { app, store };
