# Model Checker & Predicate Integration Architecture

## Overview

The modal language has two implementations:
1. **Rust** - Synchronous model checking, primarily for parsing and AST operations
2. **JavaScript** - Asynchronous execution environment used by the network

## Predicate Evaluation Strategy

### Why JS is Better Suited for Predicates

**Rust Model Checker**:
- Designed for synchronous, pure model checking
- Operates on static models without external dependencies
- Used primarily for validation and theorem proving
- Cannot easily perform async operations (WASM execution with datastore access)

**JavaScript Kripke Machine**:
- Already async (uses `await` throughout)
- Has direct access to `WasmExecutor` and network datastore
- Used by the actual network for live contract evaluation
- Natural fit for predicate evaluation with caching

### Decision

**Predicate evaluation will be implemented in the JavaScript Kripke machine**, not the Rust model checker.

## Implementation Plan

### Rust Layer (AST Only)
✅ Extended `Property` with `PropertySource` enum
✅ Added helper methods: `is_static()`, `is_predicate()`, `get_predicate()`
✅ Backward compatible with existing code

The Rust model checker will continue to work with static properties. For predicate-based properties, it will treat them as "unknown" or skip evaluation.

### JavaScript Layer (Full Integration)
🔄 Extend `Property` class with predicate support ✅
🔄 Update `PropertyTable` to evaluate predicates
🔄 Integrate `WasmExecutor` with the Kripke machine
🔄 Add predicate result caching per state

## Usage Pattern

```javascript
// In JS Kripke machine:
const property = Property.fromText('+amount_in_range({"amount":100,"min":0,"max":1000})');

if (property.isPredicate()) {
  const { path, args } = property.getPredicate();
  
  // Execute via WasmExecutor
  const result = await wasmExecutor.execute(path, args, context);
  
  // Cache result
  cache.set(property.name, result);
  
  // Convert to proposition
  const proposition = result.valid ? '+amount_in_range' : '-amount_in_range';
  
  // Use in model checking
  return proposition;
}
```

## Benefits

1. **Clean Separation**: Rust for AST/parsing, JS for execution
2. **Async-First**: JS handles async predicate evaluation naturally
3. **Performance**: JS can cache WASM modules and predicate results
4. **Pragmatic**: Follows existing architecture (network uses JS)
5. **Type Safety**: Both layers have strong typing

## Future Considerations

If synchronous predicate evaluation is needed in Rust:
- Create a blocking executor (not recommended for network use)
- Use it only for testing/validation scenarios
- Keep it separate from the main model checker

## Status

- ✅ Rust AST extended with predicate support
- ✅ JS Property class extended
- 🔄 JS PropertyTable integration (next)
- ⏳ JS WasmExecutor integration (pending)
- ⏳ Caching & performance optimization (pending)

---

**Conclusion**: The property system now supports predicates in the AST layer. Actual execution will happen in JavaScript where async operations are natural and the network infrastructure already exists.

