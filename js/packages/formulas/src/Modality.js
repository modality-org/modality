import Expression from "./language/Expression.js";

import TestFactory from "./TestFactory.js";
import Test from "./Test.js";

export const SIGNS = {
  TRUE: "+",
  FALSE: "-",
  MAYBE: "?",
};
export default class Modality {
  static SIGNS = SIGNS;

  static parseProp = TestFactory.parseProp;
  static getPropFromTest = TestFactory.getPropFromTest;
  static escapeArgs = TestFactory.escapeArgs;
  static unescapeArgs = TestFactory.unescapeArgs;

  static toModalFormula(text) {
    const expr = new Expression(text);
    return expr.toModalFormula();
  }

  constructor() {
    this.test_factories = {};
    return this;
  }

  registerTestFactory(tf) {
    this.test_factories[tf.name] = tf;
  }

  getAllTestFactories() {
    return Object.values(this.test_factories);
  }

  getTestFactory(name) {
    const tf = this.test_factories[name];
    if (!tf) {
      throw new Error(`test factory '${name}' not found`);
    }
    return tf;
  }
}
