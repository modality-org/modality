import BaseFormula from "./BaseFormula.js";
import { unionOfSets } from "@modality-dev/utils/sets";

export default class BoxFormula extends BaseFormula {
  constructor(inner, outer) {
    super();
    this.inner = inner;
    this.outer = outer;
  }

  async getValue(ctx) {
    return true; // TODO
  }

  getProps() {
    return new unionOfSets(this.inner.getProps(), this.outer.getProps());
  }

  getSignedProps() {
    return new unionOfSets(
      this.inner.getSignedProps(),
      this.outer.getSignedProps()
    );
  }

  getFreeVars(ctx) {
    return new unionOfSets(this.inner.getFreeVars(), this.outer.getFreeVars());
  }

  getBoundVars(ctx) {
    return new unionOfSets(
      this.inner.getBoundVars(),
      this.outer.getBoundVars()
    );
  }

  toText() {
    return `[${this.inner.toText()}] ${this.outer.toText()}`;
  }
}
