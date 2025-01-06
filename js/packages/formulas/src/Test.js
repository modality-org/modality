import { parseProp, getPropFromTest } from "./TestFactory.js";

export default class Test {
  constructor(name, args) {
    this.name = name;
    this.args = args;
    return this;
  }

  static fromProp(prop) {
    const { name, args } = parseProp(prop);
    return new Test({ name, args });
  }

  static toProp(name, args) {
    return getPropFromTest(name, args);
  }

  toProp() {
    Test.toProp(this.name, this.args);
  }

  async evaluate(context) {
    throw new Error("not implemented");
  }

  async correlate(other_tests = []) {
    throw new Error("not implemented");
  }
}
