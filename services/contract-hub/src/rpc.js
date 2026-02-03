/**
 * JSON-RPC handler for Contract Hub
 * 
 * Provides RPC interface for querying contracts, compatible with modal-rpc.
 */

/**
 * Create RPC handler with store access
 * @param {ContractStore} store
 * @returns {Function} Express middleware
 */
export function createRpcHandler(store) {
  return async (req, res) => {
    const { jsonrpc, id, method, params } = req.body;
    
    // Validate JSON-RPC 2.0 format
    if (jsonrpc !== '2.0') {
      return res.json(rpcError(id, -32600, 'Invalid Request: jsonrpc must be "2.0"'));
    }
    
    if (!method || typeof method !== 'string') {
      return res.json(rpcError(id, -32600, 'Invalid Request: method required'));
    }
    
    try {
      const result = await dispatch(store, method, params || {});
      return res.json(rpcSuccess(id, result));
    } catch (err) {
      if (err.code) {
        return res.json(rpcError(id, err.code, err.message, err.data));
      }
      return res.json(rpcError(id, -32603, err.message));
    }
  };
}

/**
 * Dispatch RPC method
 */
async function dispatch(store, method, params) {
  switch (method) {
    case 'getHealth':
      return getHealth(store);
      
    case 'getVersion':
      return getVersion();
      
    case 'getBlockHeight':
      return getBlockHeight(store);
      
    case 'getContract':
      return getContract(store, params);
      
    case 'getContractState':
      return getContractState(store, params);
      
    case 'getCommits':
      return getCommits(store, params);
      
    case 'getCommit':
      return getCommit(store, params);
      
    case 'submitCommit':
      return submitCommit(store, params);
      
    default:
      throw rpcException(-32601, `Method not found: ${method}`);
  }
}

// ============================================================================
// RPC Methods
// ============================================================================

function getHealth(store) {
  return {
    status: 'ok',
    version: '0.1.0',
    node_type: 'hub',
  };
}

function getVersion() {
  return '0.1.0';
}

function getBlockHeight(store) {
  // Hub doesn't have blocks, return commit count as pseudo-height
  const stats = store.getStats();
  return {
    height: stats?.totalCommits || 0,
    hash: null,
    timestamp: Date.now(),
  };
}

function getContract(store, params) {
  const { contract_id, include_commits, include_state } = params;
  
  if (!contract_id) {
    throw rpcException(-32602, 'Missing contract_id');
  }
  
  const info = store.getContract(contract_id);
  if (!info) {
    throw rpcException(-32000, 'Contract not found', { contract_id });
  }
  
  const result = {
    id: info.id,
    head: info.head,
    commit_count: info.commit_count || 0,
    created_at: info.created_at,
    updated_at: info.updated_at,
  };
  
  if (include_commits) {
    const commits = store.pullCommits(contract_id) || [];
    result.commits = commits.map(c => ({
      hash: c.hash,
      parent: c.parent,
      commit_type: c.data?.method || 'unknown',
      timestamp: c.timestamp || Date.now(),
      signer_count: c.signatures?.length || 0,
    }));
  }
  
  if (include_state) {
    result.state = store.getContractState?.(contract_id) || {};
  }
  
  return result;
}

function getContractState(store, params) {
  const { contract_id } = params;
  
  if (!contract_id) {
    throw rpcException(-32602, 'Missing contract_id');
  }
  
  const info = store.getContract(contract_id);
  if (!info) {
    throw rpcException(-32000, 'Contract not found', { contract_id });
  }
  
  return store.getContractState?.(contract_id) || {};
}

function getCommits(store, params) {
  const { contract_id, limit, before, after } = params;
  
  if (!contract_id) {
    throw rpcException(-32602, 'Missing contract_id');
  }
  
  const info = store.getContract(contract_id);
  if (!info) {
    throw rpcException(-32000, 'Contract not found', { contract_id });
  }
  
  let commits = store.pullCommits(contract_id, after) || [];
  
  // Apply limit
  const maxLimit = limit || 100;
  const hasMore = commits.length > maxLimit;
  commits = commits.slice(0, maxLimit);
  
  return {
    contract_id,
    commits: commits.map(c => ({
      hash: c.hash,
      parent: c.parent,
      commit_type: c.data?.method || 'unknown',
      path: c.data?.path,
      payload: c.data,
      timestamp: c.timestamp || Date.now(),
      signatures: (c.signatures || []).map(s => ({
        public_key: s.publicKey || s.public_key,
        signature: s.signature,
      })),
    })),
    has_more: hasMore,
  };
}

function getCommit(store, params) {
  const { contract_id, hash } = params;
  
  if (!contract_id || !hash) {
    throw rpcException(-32602, 'Missing contract_id or hash');
  }
  
  const info = store.getContract(contract_id);
  if (!info) {
    throw rpcException(-32000, 'Contract not found', { contract_id });
  }
  
  const commit = store.getCommit(contract_id, hash);
  if (!commit) {
    throw rpcException(-32002, 'Commit not found', { hash });
  }
  
  return {
    hash: commit.hash,
    parent: commit.parent,
    commit_type: commit.data?.method || 'unknown',
    path: commit.data?.path,
    payload: commit.data,
    timestamp: commit.timestamp || Date.now(),
    signatures: (commit.signatures || []).map(s => ({
      public_key: s.publicKey || s.public_key,
      signature: s.signature,
    })),
  };
}

async function submitCommit(store, params) {
  const { contract_id, commit } = params;
  
  if (!contract_id || !commit) {
    throw rpcException(-32602, 'Missing contract_id or commit');
  }
  
  const info = store.getContract(contract_id);
  if (!info) {
    throw rpcException(-32000, 'Contract not found', { contract_id });
  }
  
  // Verify parent matches head
  if (commit.parent !== info.head) {
    return {
      success: false,
      hash: commit.hash,
      error: `Invalid parent: expected ${info.head}, got ${commit.parent}`,
    };
  }
  
  try {
    const commits = [{
      hash: commit.hash,
      parent: commit.parent,
      data: commit.payload,
      timestamp: commit.timestamp,
      signatures: commit.signatures,
    }];
    
    const result = store.pushCommits(contract_id, commits);
    
    return {
      success: true,
      hash: result.head || commit.hash,
      error: null,
    };
  } catch (err) {
    return {
      success: false,
      hash: commit.hash,
      error: err.message,
    };
  }
}

// ============================================================================
// Helpers
// ============================================================================

function rpcSuccess(id, result) {
  return {
    jsonrpc: '2.0',
    id: id ?? null,
    result,
  };
}

function rpcError(id, code, message, data = null) {
  const error = { code, message };
  if (data) error.data = data;
  return {
    jsonrpc: '2.0',
    id: id ?? null,
    error,
  };
}

function rpcException(code, message, data = null) {
  const err = new Error(message);
  err.code = code;
  err.data = data;
  return err;
}

export default createRpcHandler;
