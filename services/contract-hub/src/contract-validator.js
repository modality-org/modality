/**
 * Contract Validator
 * 
 * Validates commits against the contract's governing model.
 * Replays predicate-guarded model transitions over the append-only commit log.
 */
import { Expression } from '../../../js/packages/formulas/src/index.js';

/**
 * Contract state tracker
 * Builds state from commits and validates new actions
 */
export class ContractValidator {
  constructor() {
    this.model = null;
    this.machine = null;
    this.currentStates = new Set();
    this.parties = new Map(); // path -> public_key
    this.state = new Map(); // path -> latest value
    this.rules = [];
  }
  
  /**
   * Load contract state from existing commits
   */
  loadFromCommits(commits) {
    for (const commit of commits) {
      this.applyCommit(commit, { validate: false, enforceRuleWitness: false });
    }
  }
  
  /**
   * Apply a commit (updating state, loading rules, etc.)
   */
  applyCommit(commit, { validate = true, enforceRuleWitness = true } = {}) {
    const data = commit.data;
    if (!data) return;
    
    const method = this.getMethod(data);
    const path = data.path;
    const content = data.content ?? data.value ?? data.model;

    if (validate && this.model) {
      const valid = this.validateCommit(commit);
      if (!valid.ok) {
        throw new Error(valid.error);
      }
    }
    
    switch (method) {
      case 'POST':
        // Data commit - might be party registration
        this.state.set(path, content);
        if (path?.includes('.id') || path?.includes('/parties/')) {
          this.registerParty(path, content);
        }
        break;
        
      case 'DELETE':
        this.state.delete(path);
        break;

      case 'MODEL':
        this.loadModel(path, content);
        break;

      case 'RULE':
        this.loadRule(path, content, data.model ?? data.witnessModel, { enforceWitness: enforceRuleWitness });
        break;
        
      case 'ACTION':
        break;
    }

    if (this.model && method !== 'MODEL') {
      this.advanceCommit(commit);
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
  loadRule(path, content, witnessModel, { enforceWitness = true } = {}) {
    const predicateClauses = this.extractRulePredicateClauses(content);
    const rule = {
      path,
      content,
      predicates: predicateClauses.flat(),
      predicateClauses
    };
    if (enforceWitness && rule.predicates.length > 0) {
      if (!witnessModel) {
        throw new Error('RULE requires a witness model');
      }

      const parsedWitness = this.parseWitnessModel(witnessModel);
      if (!parsedWitness) {
        throw new Error('RULE witness model is invalid');
      }

      const witnessFailure = this.validateModelAgainstRules(parsedWitness, [rule]);
      if (!witnessFailure.ok) {
        throw new Error(`RULE witness model failed: ${witnessFailure.error}`);
      }
    }

    this.rules.push(rule);
  }

  /**
   * Load a model definition.
   */
  loadModel(path, content) {
    if (typeof content !== 'string') {
      content = JSON.stringify(content);
    }
    
    // Try to parse as JSON (KripkeMachine format)
    try {
      const json = JSON.parse(content);
      if (json.systems || json.rules) {
        // KripkeMachine JSON format
        this.machine = null;
        this.model = json;
        this.currentStates = new Set(this.getInitialStates(json));
        return;
      }
    } catch {
      // Not JSON, try as Modality syntax
    }
    
    // Parse Modality syntax
    const parsed = this.parseModalitySyntax(content);
    if (parsed) {
      const ruleFailure = this.validateModelAgainstRules(parsed);
      if (!ruleFailure.ok) {
        throw new Error(ruleFailure.error);
      }
      this.model = parsed;
      this.machine = this.buildMachineFromModel(parsed);
      this.currentStates = new Set([parsed.initialState]);
    }
  }

  parseWitnessModel(content) {
    if (typeof content !== 'string') {
      content = JSON.stringify(content);
    }

    return this.parseModalitySyntax(content);
  }
  
  /**
   * Parse simple Modality model syntax.
   * Supports both:
   *   model name { state s1, s2; s1 -> s2 : ACTION }
   *   model name { initial s1; s1 -> s2 [+signed_by(/alice.id)] }
   */
  parseModalitySyntax(content) {
    const modelMatch = content.match(/model\s+(\w+)\s*\{([\s\S]*)\}/);
    if (!modelMatch) return null;
    
    const name = modelMatch[1];
    const body = modelMatch[2];
    
    const initialMatch = body.match(/\binitial\s+(\w+)/);
    const stateMatch = body.match(/state\s+([^;]+)/);
    const states = new Set(
      stateMatch
        ? stateMatch[1].split(',').map(s => s.trim()).filter(Boolean)
        : []
    );
    
    // Parse transitions
    const transitions = [];
    const transitionRegex = /(\w+)\s*->\s*(\w+)(?:\s*:\s*(\w+))?(?:\s*\[([^\]]*)\])?/g;
    let match;
    while ((match = transitionRegex.exec(body)) !== null) {
      states.add(match[1]);
      states.add(match[2]);
      transitions.push({
        from: match[1],
        to: match[2],
        action: match[3] || null,
        guard: match[4]?.trim() || null
      });
    }

    const stateList = [...states];
    
    return {
      name,
      states: stateList,
      transitions,
      initialState: initialMatch?.[1] || stateList[0] || 'init'
    };
  }
  
  /**
   * Build a machine from parsed model.
   *
   * The hub validator keeps its own replay state, so this returns null instead
   * of requiring the optional kripke-machine workspace package at runtime.
   */
  buildMachineFromModel(model) {
    return null;
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
    const activeStates = this.currentStates.size > 0 ? this.currentStates : new Set([this.model.initialState]);
    const validTransitions = this.model.transitions.filter(t => 
      activeStates.has(t.from) && t.action === action
    );
    
    if (validTransitions.length === 0) {
      return {
        ok: false,
        error: `Action '${action}' not allowed from states '${[...activeStates].join(', ')}'`
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
    
    return { ok: true, transitions: validTransitions };
  }
  
  /**
   * Take an action (update state)
   */
  takeAction(action, data = {}) {
    if (!this.model) return;
    
    this.advanceCommit({ data: { method: 'ACTION', action, ...data } });
  }
  
  /**
   * Check a guard condition
   * Guards like: +signed_by(/parties/alice.id)
   */
  checkGuard(guard, data) {
    const predicates = this.parseGuardPredicates(guard);
    for (const predicate of predicates) {
      const result = this.evaluatePredicate(predicate, { data });
      if ((predicate.sign === '+' && !result.ok) || (predicate.sign === '-' && result.ok)) {
        return { ok: false, error: result.error || `Predicate failed: ${predicate.sign}${predicate.name}` };
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
      currentStates: [...this.currentStates],
      parties: Object.fromEntries(this.parties),
      state: Object.fromEntries(this.state),
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
      .filter(t => this.currentStates.has(t.from))
      .map(t => ({
        action: t.action,
        target: t.to,
        guard: t.guard
      }));
  }

  get currentState() {
    return [...this.currentStates][0] || this.model?.initialState || null;
  }

  getMethod(data) {
    return (data?.method || data?.type || '').toUpperCase();
  }

  validateCommit(commit) {
    if (!this.model) return { ok: true };

    const data = commit.data || commit.body?.[0] || {};
    const method = this.getMethod(data);
    const activeStates = this.currentStates.size > 0 ? this.currentStates : new Set([this.model.initialState]);
    const transitions = this.model.transitions.filter(t => activeStates.has(t.from));

    for (const transition of transitions) {
      if (transition.action && !(method === 'ACTION' && transition.action === data.action)) {
        continue;
      }

      const predicateData = this.predicateData(commit);
      const guardResult = transition.guard
        ? this.checkGuard(transition.guard, predicateData)
        : { ok: true };

      if (guardResult.ok) {
        return { ok: true, transition };
      }
    }

    return {
      ok: false,
      error: `${method || 'commit'} is not allowed from states '${[...activeStates].join(', ')}'`
    };
  }

  advanceCommit(commit) {
    if (!this.model) return;

    const data = this.predicateData(commit);
    const method = this.getMethod(data);
    const activeStates = this.currentStates.size > 0 ? this.currentStates : new Set([this.model.initialState]);
    const nextStates = new Set();

    for (const transition of this.model.transitions) {
      if (!activeStates.has(transition.from)) continue;
      if (transition.action && !(method === 'ACTION' && transition.action === data.action)) continue;

      const guardResult = transition.guard
        ? this.checkGuard(transition.guard, data)
        : { ok: true };

      if (guardResult.ok) {
        nextStates.add(transition.to);
      }
    }

    if (nextStates.size > 0) {
      this.currentStates = nextStates;
    }
  }

  getInitialStates(model) {
    const systems = model.systems || [];
    return systems.flatMap(system => system.possible_current_state_ids || []);
  }

  parseGuardPredicates(guard = '') {
    const predicates = [];
    const predicateRegex = /([+-])\s*([A-Za-z_]\w*)\s*(?:\(([^)]*)\))?/g;
    let match;
    while ((match = predicateRegex.exec(guard)) !== null) {
      predicates.push({
        sign: match[1],
        name: match[2],
        args: (match[3] || '').split(',').map(arg => arg.trim()).filter(Boolean)
      });
    }
    return predicates;
  }

  evaluatePredicate(predicate, { data }) {
    const method = this.getMethod(data);
    const path = data.path || '';

    switch (predicate.name) {
      case 'signed_by':
        return this.isSignedBy(data, predicate.args[0]);
      case 'any_signed':
        return this.isAnySigned(data, predicate.args[0]);
      case 'all_signed':
        return this.isAllSigned(data, predicate.args[0]);
      case 'threshold':
        return this.isThresholdSigned(data, predicate.args[0], predicate.args[1]);
      case 'modifies':
        return { ok: this.pathMatches(path, predicate.args[0]) };
      case 'adds_rule':
        return { ok: method === 'RULE' };
      default:
        return { ok: false, error: `Unknown predicate: ${predicate.name}` };
    }
  }

  isSignedBy(data, requiredPath) {
    const requiredKey = this.parties.get(requiredPath) || this.state.get(requiredPath);
    if (!requiredKey) {
      return { ok: false, error: `Unknown party: ${requiredPath}` };
    }

    const signerKeys = this.getSignerKeys(data);
    return {
      ok: signerKeys.includes(requiredKey),
      error: `Must be signed by ${requiredPath}`
    };
  }

  isAnySigned(data, rootPath) {
    const signerKeys = new Set(this.getSignerKeys(data));
    for (const [path, key] of this.parties) {
      if (this.pathMatches(path, rootPath) && signerKeys.has(key)) {
        return { ok: true };
      }
    }
    return { ok: false, error: `Must be signed by a member under ${rootPath}` };
  }

  isAllSigned(data, rootPath) {
    const requiredKeys = [...this.parties]
      .filter(([path]) => this.pathMatches(path, rootPath))
      .map(([, key]) => key);
    const signerKeys = new Set(this.getSignerKeys(data));
    return {
      ok: requiredKeys.length > 0 && requiredKeys.every(key => signerKeys.has(key)),
      error: `Must be signed by all members under ${rootPath}`
    };
  }

  isThresholdSigned(data, count, rootPath) {
    const requiredCount = Number.parseInt(count, 10);
    if (!Number.isInteger(requiredCount) || requiredCount < 1) {
      return { ok: false, error: `Invalid threshold count: ${count}` };
    }

    const memberKeys = new Set(
      [...this.parties]
        .filter(([path]) => this.pathMatches(path, rootPath))
        .map(([, key]) => key)
    );
    const signerKeys = new Set(this.getSignerKeys(data));
    let signedCount = 0;

    for (const key of signerKeys) {
      if (memberKeys.has(key)) {
        signedCount += 1;
      }
    }

    return {
      ok: signedCount >= requiredCount,
      error: `Requires ${requiredCount} signatures under ${rootPath}`
    };
  }

  predicateData(commit) {
    const data = commit.data || commit.body?.[0] || {};
    return {
      ...data,
      signature: data.signature ?? commit.signature,
      signatures: data.signatures ?? commit.signatures
    };
  }

  getSignerKeys(data) {
    const signatures = [];
    if (data.signature) signatures.push(data.signature);
    if (Array.isArray(data.signatures)) signatures.push(...data.signatures);

    return signatures
      .map(signature => {
        if (typeof signature === 'string') return signature.split(':').at(-1);
        return signature.signer_key || signature.signerKey;
      })
      .filter(Boolean);
  }

  pathMatches(path, rootPath) {
    if (!path || !rootPath) return false;
    return path === rootPath || path.startsWith(`${rootPath.replace(/\/$/, '')}/`);
  }

  extractRulePredicates(content) {
    return this.extractRulePredicateClauses(content).flat();
  }

  extractRulePredicateClauses(content) {
    if (typeof content !== 'string') return [];
    const parsedClauses = this.extractRulePredicateClausesWithFormulaParser(content);
    if (parsedClauses) {
      return parsedClauses;
    }

    const tokens = this.tokenizeRulePredicateFormula(content);
    const ast = this.parseRulePredicateExpression(tokens);
    return ast ? this.rulePredicateAstToClauses(ast) : [];
  }

  extractRulePredicateClausesWithFormulaParser(content) {
    const formulaContent = this.extractRuleFormulaContent(content);
    if (!formulaContent) return null;
    if (/\band\b/i.test(formulaContent) && /\bor\b/i.test(formulaContent)) {
      return null;
    }
    if (/\bnot\b/i.test(formulaContent) && /\b(and|or)\b/i.test(formulaContent)) {
      return null;
    }

    try {
      const formula = new Expression(formulaContent);
      const ast = this.formulaAstToRulePredicateAst(formula);
      if (!ast) return null;
      return this.rulePredicateAstToClauses(ast);
    } catch {
      return null;
    }
  }

  extractRuleFormulaContent(content) {
    const formulaMatch = content.match(/\bformula\b/i);
    if (!formulaMatch) return content;

    const openIndex = content.indexOf('{', formulaMatch.index + formulaMatch[0].length);
    if (openIndex === -1) return null;

    let depth = 0;
    for (let index = openIndex; index < content.length; index += 1) {
      if (content[index] === '{') {
        depth += 1;
      } else if (content[index] === '}') {
        depth -= 1;
        if (depth === 0) {
          return content.slice(openIndex + 1, index).trim();
        }
      }
    }

    return null;
  }

  formulaAstToRulePredicateAst(formula) {
    if (!formula) return null;

    if (formula.inner_formula && Object.hasOwn(formula, 'until_formula')) {
      if (formula.constructor?.name !== 'AlwaysMacro') return null;
      return formula.until_formula ? null : this.formulaAstToRulePredicateAst(formula.inner_formula);
    }

    if (formula.when_formula && (formula.also_formula || formula.next_formula)) {
      const when = this.formulaAstToRulePredicateAst(formula.when_formula);
      const then = this.formulaAstToRulePredicateAst(formula.also_formula || formula.next_formula);
      return when && then ? { type: 'or', left: { type: 'not', value: when }, right: then } : null;
    }

    if (formula.left && formula.right) {
      const left = this.formulaAstToRulePredicateAst(formula.left);
      const right = this.formulaAstToRulePredicateAst(formula.right);
      if (!left || !right) return null;
      const formulaName = formula.constructor?.name;
      if (formulaName === 'AndFormula') return { type: 'and', left, right };
      if (formulaName === 'OrFormula') return { type: 'or', left, right };
      return null;
    }

    if (formula.inner && formula.outer && formula.constructor?.name === 'BoxFormula') {
      const inner = this.formulaAstToRulePredicateAst(formula.inner);
      const outer = this.formulaAstToRulePredicateAst(formula.outer);
      if (formula.outer.constructor?.name === 'TrueAtom') return { type: 'true' };
      if (inner && formula.outer.constructor?.name === 'FalseAtom') return { type: 'not', value: inner };
      return inner && outer ? { type: 'or', left: { type: 'not', value: inner }, right: outer } : null;
    }

    if (formula.inner && formula.outer && formula.constructor?.name === 'DiamondFormula') {
      const inner = this.formulaAstToRulePredicateAst(formula.inner);
      const outer = this.formulaAstToRulePredicateAst(formula.outer);
      if (inner && outer) return { type: 'and', left: inner, right: outer };
      if (inner && formula.outer.constructor?.name === 'TrueAtom') return inner;
      return null;
    }

    if (formula.formula && (formula.constructor?.name === 'CanMacro' || formula.constructor?.name === 'MustMacro')) {
      return this.formulaAstToRulePredicateAst(formula.formula);
    }

    if (formula.formula) {
      const value = this.formulaAstToRulePredicateAst(formula.formula);
      return value ? { type: 'not', value } : null;
    }

    if (Array.isArray(formula.props)) {
      const predicates = formula.props
        .map(prop => this.signedPropToRulePredicate(prop))
        .filter(Boolean)
        .map(value => ({ type: 'predicate', value }));
      if (predicates.length === 0) return null;
      return predicates.reduce((left, right) => ({ type: 'and', left, right }));
    }

    if (formula.name) {
      return {
        type: 'predicate',
        value: {
          sign: '+',
          name: formula.name,
          args: this.formulaFunctionArgs(formula.args || [])
        }
      };
    }

    if (formula.text && formula.text !== 'true' && formula.text !== 'false') {
      return {
        type: 'predicate',
        value: { sign: '+', name: formula.text, args: [] }
      };
    }

    return null;
  }

  signedPropToRulePredicate(signedProp) {
    if (!signedProp || signedProp.isMaybe?.()) return null;
    const prop = signedProp.prop;
    const sign = signedProp.sign === false ? '-' : '+';

    if (prop?.name) {
      return {
        sign,
        name: prop.name,
        args: this.formulaFunctionArgs(prop.args || [])
      };
    }

    if (typeof prop === 'string') {
      return { sign, name: prop, args: [] };
    }

    if (prop?.text) {
      return { sign, name: prop.text, args: [] };
    }

    return null;
  }

  formulaFunctionArgs(args) {
    return args.map(arg => {
      if (arg?.toFunctionArg) return arg.toFunctionArg();
      if (arg?.str !== undefined) return arg.str;
      if (typeof arg === 'string') return `"${arg}"`;
      return String(arg);
    });
  }

  tokenizeRulePredicateFormula(content) {
    const tokens = [];
    let index = 0;

    while (index < content.length) {
      const char = content[index];

      if (/\s/.test(char)) {
        index += 1;
        continue;
      }

      if (['!', '&', '|', '(', ')'].includes(char)) {
        tokens.push({ type: char });
        index += 1;
        continue;
      }

      const arrowMatch = content.slice(index).match(/^(->|=>)/);
      if (arrowMatch) {
        tokens.push({ type: 'implies' });
        index += arrowMatch[0].length;
        continue;
      }

      const operatorMatch = content.slice(index).match(/^(and|or|not|implies)\b/i);
      if (operatorMatch) {
        const type = operatorMatch[1].toLowerCase();
        tokens.push({ type: type === 'not' ? '!' : type });
        index += operatorMatch[0].length;
        continue;
      }

      if (char === '+' || char === '-') {
        const predicateMatch = content.slice(index).match(/^([+-])\s*([A-Za-z_]\w*)\s*(?:\(([^)]*)\))?/);
        if (predicateMatch) {
          tokens.push({
            type: 'predicate',
            value: {
              sign: predicateMatch[1],
              name: predicateMatch[2],
              args: (predicateMatch[3] || '').split(',').map(arg => arg.trim()).filter(Boolean)
            }
          });
          index += predicateMatch[0].length;
          continue;
        }
      }

      const barePredicateMatch = content.slice(index).match(/^([A-Za-z_]\w*)\s*(?:\(([^)]*)\))?/);
      if (barePredicateMatch) {
        if (this.isRulePredicateName(barePredicateMatch[1], barePredicateMatch[2])) {
          tokens.push({
            type: 'predicate',
            value: {
              sign: '+',
              name: barePredicateMatch[1],
              args: (barePredicateMatch[2] || '').split(',').map(arg => arg.trim()).filter(Boolean)
            }
          });
          index += barePredicateMatch[0].length;
          continue;
        }

        index += barePredicateMatch[1].length;
        continue;
      }

      index += 1;
    }

    return tokens;
  }

  isRulePredicateName(name, argText) {
    const ignoredNames = new Set([
      'always',
      'eventually',
      'until',
      'lfp',
      'gfp',
      'when',
      'also',
      'next',
      'rule',
      'formula',
      'model',
      'initial'
    ]);
    return !ignoredNames.has(name.toLowerCase()) && (argText !== undefined || name === 'adds_rule');
  }

  parseRulePredicateExpression(tokens) {
    let position = 0;

    const parseImplies = () => {
      let node = parseOr();
      if (tokens[position]?.type === 'implies') {
        position += 1;
        node = { type: 'or', left: { type: 'not', value: node }, right: parseImplies() };
      }
      return node;
    };

    const parseOr = () => {
      let node = parseAnd();
      while (tokens[position]?.type === '|' || tokens[position]?.type === 'or') {
        position += 1;
        node = { type: 'or', left: node, right: parseAnd() };
      }
      return node;
    };

    const parseAnd = () => {
      let node = parseUnary();
      while (tokens[position]?.type === '&' || tokens[position]?.type === 'and') {
        position += 1;
        node = { type: 'and', left: node, right: parseUnary() };
      }
      return node;
    };

    const parseUnary = () => {
      if (tokens[position]?.type === '!') {
        position += 1;
        return { type: 'not', value: parseUnary() };
      }

      return parsePrimary();
    };

    const parsePrimary = () => {
      const token = tokens[position];
      if (!token) return null;

      if (token.type === 'predicate') {
        position += 1;
        return { type: 'predicate', value: token.value };
      }

      if (token.type === '(') {
        position += 1;
        const node = parseImplies();
        if (tokens[position]?.type === ')') {
          position += 1;
        }
        return node;
      }

      position += 1;
      return parsePrimary();
    };

    return parseImplies();
  }

  rulePredicateAstToClauses(ast) {
    if (!ast) return [];

    switch (ast.type) {
      case 'predicate':
        return [[ast.value]];
      case 'not':
        return this.negateRulePredicateAst(ast.value);
      case 'and': {
        const left = this.rulePredicateAstToClauses(ast.left);
        const right = this.rulePredicateAstToClauses(ast.right);
        if (left.length === 0) return right;
        if (right.length === 0) return left;
        return left.flatMap(leftClause =>
          right.map(rightClause => [...leftClause, ...rightClause])
        );
      }
      case 'or':
        return [
          ...this.rulePredicateAstToClauses(ast.left),
          ...this.rulePredicateAstToClauses(ast.right)
        ];
      case 'true':
        return [];
      default:
        return [];
    }
  }

  negateRulePredicateAst(ast) {
    if (!ast) return [];

    switch (ast.type) {
      case 'predicate':
        return [[{
          ...ast.value,
          sign: ast.value.sign === '+' ? '-' : '+'
        }]];
      case 'not':
        return this.rulePredicateAstToClauses(ast.value);
      case 'and':
        return [
          ...this.negateRulePredicateAst(ast.left),
          ...this.negateRulePredicateAst(ast.right)
        ];
      case 'or': {
        const left = this.negateRulePredicateAst(ast.left);
        const right = this.negateRulePredicateAst(ast.right);
        if (left.length === 0) return right;
        if (right.length === 0) return left;
        return left.flatMap(leftClause =>
          right.map(rightClause => [...leftClause, ...rightClause])
        );
      }
      default:
        return [];
    }
  }

  validateModelAgainstRules(model, extraRules = []) {
    const rules = [...this.rules, ...extraRules]
      .map(rule => rule.predicateClauses?.length
        ? rule.predicateClauses
        : (rule.predicates || []).map(predicate => [predicate]))
      .filter(clauses => clauses.length > 0);
    if (rules.length === 0) {
      return { ok: true };
    }

    if (!model.transitions || model.transitions.length === 0) {
      return {
        ok: false,
        error: 'MODEL has no transitions to satisfy existing rule predicates'
      };
    }

    for (const transition of model.transitions) {
      const guardPredicates = this.parseGuardPredicates(transition.guard || '');
      for (const clauses of rules) {
        const satisfied = clauses.some(clause =>
          clause.every(required => this.guardHasPredicate(guardPredicates, required))
        );
        if (!satisfied) {
          const description = clauses
            .map(clause => clause.map(predicate => this.formatPredicate(predicate)).join(' & '))
            .join(' | ');
          return {
            ok: false,
            error: `MODEL transition ${transition.from}->${transition.to} does not satisfy existing rule predicate ${description}`
          };
        }
      }
    }

    return { ok: true };
  }

  guardHasPredicate(guardPredicates, required) {
    return guardPredicates.some(candidate =>
      candidate.sign === required.sign &&
      candidate.name === required.name &&
      JSON.stringify(candidate.args) === JSON.stringify(required.args)
    );
  }

  formatPredicate(predicate) {
    const args = predicate.args.length > 0 ? `(${predicate.args.join(', ')})` : '';
    return `${predicate.sign}${predicate.name}${args}`;
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
      
      // Check every governed commit against the current model.
      if (validator.model) {
        const validation = validator.validateCommit(commit);
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
