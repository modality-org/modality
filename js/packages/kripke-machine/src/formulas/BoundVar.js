import BaseFormula from "./BaseFormula.js";

export default class BoundVar extends BaseFormula {
  constructor(name) {
    super();
    this.name = name;
  }

  async getValue(ctx) {
    return true; // TODO
  }

  getProps() {
    return new Set();
  }

  getSignedProps() {
    return new Set();
  }

  getFreeVars() {
    return new Set();
  }

  getBoundVars() {
    return new Set(this.name);
  }

  toText() {
    return `${this.name}`;
  }
}
