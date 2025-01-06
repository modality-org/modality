import BaseFormula from "./BaseFormula.js";
import { unionOfSets, setMinus } from "@modality-dev/utils/sets";

export default class LfpFormula extends BaseFormula {
  constructor(bound_var, inner) {
    super();
    this.bound_var = bound_var;
    this.inner = inner;
  }

  async getValue(ctx) {
    return true; // TODO
  }

  getProps(ctx) {
    return this.inner.getProps();
  }

  getSignedProps(ctx) {
    return this.inner.getSignedProps();
  }

  getFreeVars(ctx) {
    return new setMinus(this.inner.getFreeVars(), this.bound_var.toText());
  }

  getBoundVars(ctx) {
    return new unionOfSets(this.inner.getBoundVars(), this.bound_var.toText());
  }

  toText() {
    return `lfp(${this.bound_var.toText()}, ${this.inner.toText()})`;
  }
}
