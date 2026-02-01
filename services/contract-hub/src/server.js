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
// ACCESS KEY MANAGEMENT
// ============================================================================

/**
 * Register a new access key
 * POST /access/register
 * Body: { public_key: hex }
 * Returns: { access_id, public_key }
 */
app.post('/access/register', async (req, res) => {
  try {
    const { public_key } = req.body;
    if (!public_key || typeof public_key !== 'string') {
      return res.status(400).json({ error: 'public_key required (hex string)' });
    }
    
    // Validate hex format
    if (!/^[0-9a-fA-F]{64}$/.test(public_key)) {
      return res.status(400).json({ error: 'public_key must be 64 hex chars (32 bytes)' });
    }
    
    const accessId = store.registerAccess(public_key.toLowerCase());
    res.json({ access_id: accessId, public_key: public_key.toLowerCase() });
  } catch (err) {
    res.status(500).json({ error: err.message });
  }
});

/**
 * Get access info
 * GET /access/:accessId
 */
app.get('/access/:accessId', (req, res) => {
  const info = store.getAccess(req.params.accessId);
  if (!info) {
    return res.status(404).json({ error: 'Access not found' });
  }
  res.json(info);
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
    const contractId = store.createContract(req.accessId, { name, description });
    res.status(201).json({ contract_id: contractId, owner: req.accessId });
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
  const contracts = store.listContracts(req.accessId);
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
  
  // Check access
  if (info.owner !== req.accessId && !info.readers?.includes(req.accessId)) {
    return res.status(403).json({ error: 'Access denied' });
  }
  
  res.json(info);
});

/**
 * Push commits to a contract
 * POST /contracts/:contractId/push
 * Auth: Signature header (must be owner or have write access)
 * Body: { commits: [{ hash, data, parent?, signature? }] }
 */
app.post('/contracts/:contractId/push', auth.verify(), async (req, res) => {
  try {
    const { contractId } = req.params;
    const { commits } = req.body;
    
    if (!commits || !Array.isArray(commits)) {
      return res.status(400).json({ error: 'commits array required' });
    }
    
    const info = store.getContract(contractId);
    if (!info) {
      return res.status(404).json({ error: 'Contract not found' });
    }
    
    // Check write access
    if (info.owner !== req.accessId && !info.writers?.includes(req.accessId)) {
      return res.status(403).json({ error: 'Write access denied' });
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
  
  // Check read access
  if (info.owner !== req.accessId && !info.readers?.includes(req.accessId)) {
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
 * Grant access to a contract
 * POST /contracts/:contractId/access
 * Auth: Signature header (must be owner)
 * Body: { access_id, permission: 'read' | 'write' }
 */
app.post('/contracts/:contractId/access', auth.verify(), (req, res) => {
  try {
    const { contractId } = req.params;
    const { access_id, permission } = req.body;
    
    if (!access_id || !permission) {
      return res.status(400).json({ error: 'access_id and permission required' });
    }
    
    if (!['read', 'write'].includes(permission)) {
      return res.status(400).json({ error: 'permission must be "read" or "write"' });
    }
    
    const info = store.getContract(contractId);
    if (!info) {
      return res.status(404).json({ error: 'Contract not found' });
    }
    
    if (info.owner !== req.accessId) {
      return res.status(403).json({ error: 'Only owner can grant access' });
    }
    
    store.grantAccess(contractId, access_id, permission);
    res.json({ granted: true, access_id, permission });
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
