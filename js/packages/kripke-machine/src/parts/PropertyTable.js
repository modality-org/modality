import Property from "./Property.js";

export default class PropertyTable {
  constructor(default_value) {
    this.default_value = default_value;
    this.name_value = {};
  }

  clone() {
    const pt = new PropertyTable();
    pt.default_value = this.default_value;
    pt.name_value = { ...this.name_value };
    return pt;
  }

  keys() {
    return Object.keys(this.name_value);
  }

  has(name) {
    return typeof this.name_value[name] !== "undefined";
  }

  get(name) {
    if (
      typeof this.default_value !== "undefined" &&
      typeof this.name_value[name] === "undefined"
    ) {
      return this.default_value;
    } else {
      return this.name_value[name];
    }
  }

  static fromText(text, default_value) {
    const pt = new PropertyTable(default_value);
    const name_value = text
      .replace(/-\W*/, " -")
      .replace(/\+\W*/g, " +")
      .split(/ /)
      .filter((i) => i.length)
      .map((t) => Property.fromText(t));
    for (const prop of name_value) {
      if (pt.has(prop.name) && pt.get(prop.name) !== prop.value) {
        throw new Error("inconsistent property set");
      }
      pt.name_value[prop.name] = prop.value;
    }
    return pt;
  }

  toText() {
    return Object.entries(this.name_value)
      .map(([name, value]) => Property.toText(name, value))
      .join(" ");
  }
}
