/**
 * Contract Validator
 * 
 * Validates commits against the contract's governing model.
 * Uses KripkeMachine to verify state transitions are valid.
 */

import KripkeMachine from '@modality-dev/kripke-machine';

/**
 * Contract state tracker
 * Builds state from commits and validates new actions
 */
export class ContractValidator {
  constructor() {
    this.model = null;
    this.machine = null;
    this.currentState = null;
    this.parties = new Map(); // path -> public_key
    this.rules = [];
  }
  
  /**
   * Load contract state from existing commits
   */
  loadFromCommits(commits) {
    for (const commit of commits) {
      this.applyCommit(commit, { validate: false });
    }
  }
  
  /**
   * Apply a commit (updating state, loading rules, etc.)
   */
  applyCommit(commit, { validate = true } = {}) {
    const data = commit.data;
    if (!data) return;
    
    const method = data.method || data.type;
    const path = data.path;
    const content = data.content;
    
    switch (method) {
      case 'POST':
        // Data commit - might be party registration
        if (path?.includes('.id') || path?.includes('/parties/')) {
          this.registerParty(path, content);
        }
        break;
        
      case 'RULE':
        // Rule/model commit
        this.loadRule(path, content);
        break;
        
      case 'ACTION':
        // Domain action - validate against model
        if (validate && this.machine) {
          const valid = this.validateAction(data.action, data);
          if (!valid.ok) {
            throw new Error(`Invalid action '${data.action}': ${valid.error}`);
          }
        }
        this.takeAction(data.action, data);
        break;
    }
  }
  
  /**
   * Register a party (identity -> public key mapping)
   */
  registerParty(path, publicKey) {
    this.parties.set(path, publicKey);
  }
  
  /**
   * Load a rule/model definition
   */
  loadRule(path, content) {
    if (typeof content !== 'string') {
      content = JSON.stringify(content);
    }
    
    // Try to parse as JSON (KripkeMachine format)
    try {
      const json = JSON.parse(content);
      if (json.systems || json.rules) {
        // KripkeMachine JSON format
        this.machine = KripkeMachine.fromJSON(json);
        this.model = json;
        return;
      }
    } catch {
      // Not JSON, try as Modality syntax
    }
    
    // Parse Modality syntax
    const parsed = this.parseModalitySyntax(content);
    if (parsed) {
      this.model = parsed;
      this.machine = this.buildMachineFromModel(parsed);
    }
    
    this.rules.push({ path, content });
  }
  
  /**
   * Parse simple Modality model syntax
   * model name { state s1, s2; s1 -> s2 : ACTION }
   */
  parseModalitySyntax(content) {
    const modelMatch = content.match(/model\s+(\w+)\s*\{([\s\S]*)\}/);
    if (!modelMatch) return null;
    
    const name = modelMatch[1];
    const body = modelMatch[2];
    
    // Parse states
    const stateMatch = body.match(/state\s+([^;]+)/);
    const states = stateMatch 
      ? stateMatch[1].split(',').map(s => s.trim())
      : [];
    
    // Parse transitions
    const transitions = [];
    const transitionRegex = /(\w+)\s*->\s*(\w+)\s*:\s*(\w+)(?:\s*\[([^\]]+)\])?/g;
    let match;
    while ((match = transitionRegex.exec(body)) !== null) {
      transitions.push({
        from: match[1],
        to: match[2],
        action: match[3],
        guard: match[4] || null
      });
    }
    
    return {
      name,
      states,
      transitions,
      initialState: states[0] || 'init'
    };
  }
  
  /**
   * Build a KripkeMachine from parsed model
   */
  buildMachineFromModel(model) {
    // Convert to KripkeMachine JSON format
    const kmJson = {
      systems: [{
        states: {},
        arrows: [],
        possible_current_state_ids: [model.initialState]
      }],
      rules: []
    };
    
    // Add states
    for (const state of model.states) {
      kmJson.systems[0].states[state] = { id: state };
    }
    
    // Add transitions as arrows
    for (const t of model.transitions) {
      kmJson.systems[0].arrows.push({
        source: t.from,
        target: t.to,
        properties: { action: t.action }
      });
    }
    
    this.currentState = model.initialState;
    
    try {
      return KripkeMachine.fromJSON(kmJson);
    } catch (err) {
      console.warn('Could not build KripkeMachine:', err.message);
      return null;
    }
  }
  
  /**
   * Validate an action against the current model/state
   */
  validateAction(action, data = {}) {
    if (!this.model) {
      // No model loaded - allow all actions
      return { ok: true };
    }
    
    // Check if action is valid from current state
    const validTransitions = this.model.transitions.filter(t => 
      t.from === this.currentState && t.action === action
    );
    
    if (validTransitions.length === 0) {
      return {
        ok: false,
        error: `Action '${action}' not allowed from state '${this.currentState}'`
      };
    }
    
    // Check guards (signature requirements)
    for (const t of validTransitions) {
      if (t.guard) {
        const guardResult = this.checkGuard(t.guard, data);
        if (!guardResult.ok) {
          return guardResult;
        }
      }
    }
    
    // If using KripkeMachine, do full verification
    if (this.machine) {
      try {
        const step = { properties_text: action };
        const [canTake, error] = this.machine.canTakeStep(step);
        if (!canTake) {
          return { ok: false, error: error || 'Invalid transition' };
        }
      } catch (err) {
        return { ok: false, error: err.message };
      }
    }
    
    return { ok: true, transitions: validTransitions };
  }
  
  /**
   * Take an action (update state)
   */
  takeAction(action, data = {}) {
    if (!this.model) return;
    
    const transition = this.model.transitions.find(t =>
      t.from === this.currentState && t.action === action
    );
    
    if (transition) {
      this.currentState = transition.to;
      
      // Update KripkeMachine if present
      if (this.machine) {
        try {
          const step = { properties_text: action };
          this.machine.takeStep(step);
        } catch {
          // Ignore machine errors after state update
        }
      }
    }
  }
  
  /**
   * Check a guard condition
   * Guards like: +signed_by(/parties/alice.id)
   */
  checkGuard(guard, data) {
    // Parse signed_by requirement
    const signedByMatch = guard.match(/\+?signed_by\(([^)]+)\)/);
    if (signedByMatch) {
      const requiredPath = signedByMatch[1];
      const requiredKey = this.parties.get(requiredPath);
      
      if (!requiredKey) {
        return { ok: false, error: `Unknown party: ${requiredPath}` };
      }
      
      // Check if commit is signed by required party
      if (data.signature) {
        const signerKey = data.signature.signer_key || data.signature.signerKey;
        if (signerKey !== requiredKey) {
          return { ok: false, error: `Must be signed by ${requiredPath}` };
        }
      } else {
        return { ok: false, error: `Action requires signature from ${requiredPath}` };
      }
    }
    
    return { ok: true };
  }
  
  /**
   * Get current contract state
   */
  getState() {
    return {
      currentState: this.currentState,
      parties: Object.fromEntries(this.parties),
      model: this.model,
      rulesCount: this.rules.length
    };
  }
  
  /**
   * Get valid actions from current state
   */
  getValidActions() {
    if (!this.model) return [];
    
    return this.model.transitions
      .filter(t => t.from === this.currentState)
      .map(t => ({
        action: t.action,
        target: t.to,
        guard: t.guard
      }));
  }
}

/**
 * Parse a REPOST path in format $contract_id:/path
 * Returns { contractId, remotePath } or null if invalid
 */
export function parseRepostPath(path) {
  if (!path || !path.startsWith('$')) return null;
  
  const colonPos = path.indexOf(':/');
  if (colonPos === -1) return null;
  
  const contractId = path.substring(1, colonPos);
  const remotePath = path.substring(colonPos + 1);
  
  if (!contractId || !remotePath || !remotePath.startsWith('/')) {
    return null;
  }
  
  return { contractId, remotePath };
}

/**
 * Validate a REPOST commit against the source contract's latest state
 * Hub/network responsibility: only allow reposting latest values
 */
export async function validateRepost(store, commit) {
  const data = commit.data || commit.body?.[0];
  if (!data) {
    return { ok: false, error: 'Missing commit data' };
  }
  
  const method = (data.method || '').toLowerCase();
  if (method !== 'repost') {
    return { ok: true }; // Not a repost, skip
  }
  
  const path = data.path;
  const value = data.value;
  
  // Parse the repost path
  const parsed = parseRepostPath(path);
  if (!parsed) {
    return { 
      ok: false, 
      error: `Invalid REPOST path format: ${path}. Expected $contract_id:/path` 
    };
  }
  
  const { contractId: sourceContractId, remotePath } = parsed;
  
  // Fetch the source contract's current state
  const sourceCommits = store.pullCommits(sourceContractId);
  if (!sourceCommits || sourceCommits.length === 0) {
    return {
      ok: false,
      error: `Source contract '${sourceContractId}' not found or has no commits`
    };
  }
  
  // Build source contract state
  const sourceState = buildContractState(sourceCommits);
  
  // Get the value at the remote path
  const normalizedPath = remotePath.startsWith('/') ? remotePath.substring(1) : remotePath;
  const sourceValue = sourceState[normalizedPath] ?? sourceState[remotePath];
  
  if (sourceValue === undefined) {
    return {
      ok: false,
      error: `Path '${remotePath}' not found in source contract '${sourceContractId}'`
    };
  }
  
  // Compare values (deep equality for objects)
  const valuesMatch = JSON.stringify(sourceValue) === JSON.stringify(value);
  
  if (!valuesMatch) {
    return {
      ok: false,
      error: `REPOST value does not match source contract's latest value at '${remotePath}'`
    };
  }
  
  return { ok: true };
}

/**
 * Build contract state from commits (for REPOST validation)
 */
function buildContractState(commits) {
  const state = {};
  
  for (const commit of commits) {
    const body = commit.body || [commit.data].filter(Boolean);
    
    for (const action of body) {
      const method = (action.method || action.type || '').toLowerCase();
      const path = action.path;
      const value = action.value ?? action.content;
      
      if (path && ['post', 'genesis', 'rule', 'repost'].includes(method)) {
        // Normalize path (remove leading slash for storage)
        const normalizedPath = path.startsWith('/') ? path.substring(1) : path;
        state[normalizedPath] = value;
        state[path] = value; // Also store with original path
      }
    }
  }
  
  return state;
}

/**
 * Validate commits against contract model
 */
export async function validateContractLogic(store, contractId, newCommits) {
  const errors = [];
  
  // Load existing commits
  const existingCommits = store.pullCommits(contractId);
  
  // Create validator and load existing state
  const validator = new ContractValidator();
  validator.loadFromCommits(existingCommits);
  
  // Validate each new commit
  for (let i = 0; i < newCommits.length; i++) {
    const commit = newCommits[i];
    const prefix = `commits[${i}]`;
    
    try {
      const data = commit.data || commit.body?.[0];
      const method = (data?.method || data?.type || '').toLowerCase();
      
      // Validate REPOST commits against source contract
      if (method === 'repost') {
        const repostValidation = await validateRepost(store, commit);
        if (!repostValidation.ok) {
          errors.push(`${prefix}: ${repostValidation.error}`);
          continue;
        }
      }
      
      // Check if this is an ACTION commit
      if (method === 'action') {
        const validation = validator.validateAction(data.action, data);
        if (!validation.ok) {
          errors.push(`${prefix}: ${validation.error}`);
          continue;
        }
      }
      
      // Apply commit to update validator state
      validator.applyCommit(commit, { validate: false });
      
    } catch (err) {
      errors.push(`${prefix}: ${err.message}`);
    }
  }
  
  return {
    valid: errors.length === 0,
    errors,
    state: validator.getState(),
    validActions: validator.getValidActions()
  };
}

export default ContractValidator;
