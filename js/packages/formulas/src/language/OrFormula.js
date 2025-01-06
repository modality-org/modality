import BaseFormula from "./BaseFormula.js";
import NotFormula from "./NotFormula.js";
import AndFormula from "./AndFormula.js";
export default class OrFormula extends BaseFormula {
  constructor(left, right) {
    super();
    this.left = left;
    this.right = right;
  }

  async getValue(ctx) {
    return (await this.left.getValue(ctx)) || (await this.right.getValue(ctx));
  }

  expandFunctions() {
    const left = this.left.expandFunctions();
    const right = this.right.expandFunctions();
    return {
      constraint: `${left.constraint} or ${right.constraint}`,
      functions: {
        ...left.functions,
        ...right.functions,
      },
    };
  }

  toModalFormula() {
    return `${this.left.toModalFormula()} or ${this.right.toModalFormula()}`;
  }

  negated() {
    return new AndFormula(
      new NotFormula(this.left),
      new NotFormula(this.right)
    );
  }
}
