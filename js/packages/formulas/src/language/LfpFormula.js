import BaseFormula from "./BaseFormula.js";
import { unionOfSets, setMinus } from "@modality-dev/utils/sets";

export default class LfpFormula extends BaseFormula {
  constructor(state_set_variable, inner) {
    super();
    this.state_set_variable = state_set_variable;
    this.inner = inner;
  }

  async getValue(ctx) {
    return true; // TODO
  }

  getFreeVars(ctx) {
    return new setMinus(
      this.inner.getFreeVars(),
      this.state_set_variable.toText()
    );
  }

  getBoundVars(ctx) {
    return new unionOfSets(
      this.inner.getBoundVars(),
      this.state_set_variable.toText()
    );
  }

  toText() {
    return `lfp(${this.state_set_variable.toText()}, ${this.inner.toText()})`;
  }

  toModalFormula() {
    return `lfp(${this.state_set_variable.toModalFormula()}, ${this.inner.toModalFormula()})`;
  }
}
