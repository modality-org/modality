import BaseFormula from "./BaseFormula.js";

export default class TrueAtom extends BaseFormula {
  constructor() {
    super();
  }

  async getValue(ctx) {
    return true;
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
    return `true`;
  }
}
