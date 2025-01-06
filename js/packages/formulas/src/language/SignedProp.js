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

  negated() {
    if (this.sign === true) {
      return new SignedProp(false, this.prop);
    } else if (this.sign === false) {
      return new SignedProp(true, this.prop);
    } else {
      return new SignedProp(null, this.prop);
    }
  }

  async getValue() {
    return true; // TODO
  }

  getFreeProps() {
    return new Set([this.prop]);
  }

  signToText() {
    if (this.sign === true) {
      return "+";
    } else if (this.sign === false) {
      return "-";
    } else {
      return "?";
    }
  }

  isMaybe() {
    return this.signToText() === "?";
  }

  toText() {
    if (this.isMaybe()) {
      return "";
    }
    if (this.prop.toModalFormula) {
      return `${this.signToText()}${this.prop.toModalFormula()}`;
    } else {
      return `${this.signToText()}${this.prop}`;
    }
  }

  toUnsignedText() {
    if (this.isMaybe()) {
      return "";
    }
    if (this.prop.toModalFormula) {
      return `${this.prop.toModalFormula()}`;
    } else {
      return `${this.prop}`;
    }
  }
}
