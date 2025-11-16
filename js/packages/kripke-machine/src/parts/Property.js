export default class Property {
  constructor(name, value = true, source = null) {
    this.name = name;
    this.value = value;
    this.source = source; // null for static, { type: 'predicate', path: '...', args: {...} } for predicate
    return this;
  }

  static fromText(text) {
    const m = text.match(/([-+])?(.+)/);
    const name = m[2];
    const value = m[1] === "-" ? false : true;
    
    // Check if this is a predicate call: name(args)
    const predicateMatch = name.match(/^([a-z_]+)\((.*)\)$/);
    if (predicateMatch) {
      const predicateName = predicateMatch[1];
      const argsText = predicateMatch[2];
      
      try {
        const args = argsText ? JSON.parse(argsText) : {};
        return new Property(predicateName, value, {
          type: 'predicate',
          path: `/_code/modal/${predicateName}.wasm`,
          args: args
        });
      } catch (e) {
        // If JSON parsing fails, treat as static property
        return new Property(name, value, { type: 'static' });
      }
    }
    
    return new Property(name, value, { type: 'static' });
  }

  static toText(name, value) {
    return `${value ? "+" : "-"}${name}`;
  }

  toText() {
    return Property.toText(this.name, this.value);
  }

  static arrayFromText(text) {
    return text
      .replace(/-\W*/, " -")
      .replace(/\+\W*/g, " +")
      .split(/ /)
      .filter((i) => i.length)
      .map((t) => Property.fromText(t));
  }

  /// Check if this is a static property
  isStatic() {
    return this.source === null || this.source?.type === 'static';
  }

  /// Check if this is a predicate-based property
  isPredicate() {
    return this.source?.type === 'predicate';
  }

  /// Get predicate information if this is a predicate property
  getPredicate() {
    if (this.isPredicate()) {
      return {
        path: this.source.path,
        args: this.source.args
      };
    }
    return null;
  }
}
