import BaseFormula from "./BaseFormula.js";

export default class StateSetVariable extends BaseFormula {
  constructor(name) {
    super();
    this.name = name;
  }

  async getValue(ctx) {
    return true; // TODO
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

  toModalFormula() {
    return `${this.name}`;
  }
}
