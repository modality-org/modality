import BaseFormula from "./BaseFormula.js";
import { unionOfSets } from "@modality-dev/utils/sets";

export default class DiamondFormula extends BaseFormula {
  constructor(inner, outer) {
    super();
    this.inner = inner;
    this.outer = outer;
  }

  async getValue(ctx) {
    return true; // TODO
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
    return `<${this.inner.toText()}> ${this.outer.toText()}`;
  }

  toModalFormula() {
    return `<${this.inner.toModalFormula({
      filterMaybe: true,
    })}> ${this.outer.toModalFormula()}`;
  }
}
