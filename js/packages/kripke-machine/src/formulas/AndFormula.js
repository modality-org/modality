import BaseFormula from "./BaseFormula.js";
import { unionOfSets } from "@modality-dev/utils/sets";

export default class AndFormula extends BaseFormula {
  constructor(left, right) {
    super();
    this.left = left;
    this.right = right;
  }

  async getValue(ctx) {
    return (await this.left.getValue(ctx)) && (await this.right.getValue(ctx));
  }

  getProps() {
    return new unionOfSets(this.left.getProps(), this.right.getProps());
  }

  getSignedProps() {
    return new unionOfSets(
      this.left.getSignedProps(),
      this.right.getSignedProps()
    );
  }

  getFreeVars() {
    return new unionOfSets(this.left.getFreeVars(), this.right.getFreeVars());
  }

  getBoundVars() {
    return new unionOfSets(this.left.getBoundVars(), this.right.getBoundVars());
  }

  toText() {
    return `${this.left.toText()} and ${this.right.toText()}`;
  }
}
