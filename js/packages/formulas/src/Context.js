export default class Context {
  constructor(values = {}, funcs = {}) {
    this.values = values;
    this.funcs = funcs;
    return this;
  }

  setValue(key, value) {
    this.values[key] = value;
  }

  getValues() {
    return this.values;
  }

  async callFunction(name, args) {
    //
    if (this.funcs[name]) {
      return await this.callBuiltinFunction(name, args);
    } else {
      throw new Error(`Unknown function: ${name}`);
    }
  }

  async callBuiltinFunction(name, args) {
    const func = this.funcs[name];
    const ctx = this.getValues();
    return await func.withArgs(...(args || [])).evaluate(ctx);
  }

  callUserFunction(name, args) {}
}
