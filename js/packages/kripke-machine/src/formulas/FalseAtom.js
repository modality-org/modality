import BaseFormula from "./BaseFormula.js";

export default class FalseAtom extends BaseFormula {
  constructor() {
    super();
  }

  async getValue(ctx) {
    return false;
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
    return new Set();
  }

  toText() {
    return `false`;
  }
}
