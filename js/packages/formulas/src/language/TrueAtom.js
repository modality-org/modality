import BaseFormula from "./BaseFormula.js";
import FalseAtom from "./FalseAtom.js";
export default class TrueAtom extends BaseFormula {
  constructor() {
    super();
  }

  async getValue(ctx) {
    return true;
  }

  expandFunctions() {
    return {
      constraint: `true`,
      functions: {},
    };
  }

  toModalFormula() {
    return `true`;
  }

  negated() {
    return new FalseAtom();
  }
}
