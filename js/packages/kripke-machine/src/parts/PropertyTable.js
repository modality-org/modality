import Property from "./Property.js";

export default class PropertyTable {
  constructor(default_value, predicateExecutor = null) {
    this.default_value = default_value;
    this.name_value = {};
    this.predicateExecutor = predicateExecutor;
    this.predicateCache = new Map(); // Cache for predicate evaluation results
  }

  clone() {
    const pt = new PropertyTable(this.default_value, this.predicateExecutor);
    pt.name_value = { ...this.name_value };
    pt.predicateCache = new Map(this.predicateCache);
    return pt;
  }

  keys() {
    return Object.keys(this.name_value);
  }

  has(name) {
    return typeof this.name_value[name] !== "undefined";
  }

  get(name) {
    if (
      typeof this.default_value !== "undefined" &&
      typeof this.name_value[name] === "undefined"
    ) {
      return this.default_value;
    } else {
      return this.name_value[name];
    }
  }

  /// Get a property value, evaluating predicates if necessary
  /// Returns { value: boolean, wasPredicate: boolean }
  async getValue(name, context = {}) {
    // Check if we have a cached value
    if (this.has(name)) {
      return { value: this.get(name), wasPredicate: false };
    }

    // Check if we have a predicate in the cache
    const cacheKey = `${name}_${JSON.stringify(context)}`;
    if (this.predicateCache.has(cacheKey)) {
      const cachedValue = this.predicateCache.get(cacheKey);
      return { value: cachedValue, wasPredicate: true };
    }

    // If we have a predicate executor, try to evaluate
    if (this.predicateExecutor) {
      // TODO: Implement predicate execution here
      // This will be implemented when we integrate with the network layer
      // For now, return default value
    }

    // Return default value
    if (typeof this.default_value !== "undefined") {
      return { value: this.default_value, wasPredicate: false };
    }

    return { value: undefined, wasPredicate: false };
  }

  /// Set a predicate result in the cache
  setPredicateResult(name, value, context = {}) {
    const cacheKey = `${name}_${JSON.stringify(context)}`;
    this.predicateCache.set(cacheKey, value);
  }

  /// Clear predicate cache
  clearPredicateCache() {
    this.predicateCache.clear();
  }

  static fromText(text, default_value, predicateExecutor = null) {
    const pt = new PropertyTable(default_value, predicateExecutor);
    const name_value = text
      .replace(/-\W*/, " -")
      .replace(/\+\W*/g, " +")
      .split(/ /)
      .filter((i) => i.length)
      .map((t) => Property.fromText(t));
    for (const prop of name_value) {
      if (pt.has(prop.name) && pt.get(prop.name) !== prop.value) {
        throw new Error("inconsistent property set");
      }
      pt.name_value[prop.name] = prop.value;
    }
    return pt;
  }

  toText() {
    return Object.entries(this.name_value)
      .map(([name, value]) => Property.toText(name, value))
      .join(" ");
  }
}
