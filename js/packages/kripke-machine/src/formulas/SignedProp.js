export default class SignedProp {
  constructor(sign, prop) {
    this.sign = sign;
    if (prop === "*") {
      this.prop = "*";
      this.type = "star";
    } else {
      this.prop = prop;
      this.type = "prop";
    }
    return this;
  }

  async getValue() {
    return true; // TODO
  }

  getProps() {
    return new Set([this.prop]);
  }

  getSignedProps() {
    return new Set([`${this.sign ? "+" : "-"}${this.prop}`]);
  }

  getFreeProps() {
    return new Set([this.prop]);
  }

  toText() {
    return `${this.sign ? "+" : "-"}${this.prop}`;
  }
}
