export default class Property {
  constructor(name, value = true) {
    this.name = name;
    this.value = value;
    return this;
  }

  static fromText(text) {
    const m = text.match(/([-+])?(.+)/);
    const name = m[2];
    const value = m[1] === "-" ? false : true;
    return new Property(name, value);
  }

  static toText(name, value) {
    return `${value ? "+" : "-"}${name}`;
  }

  toText() {
    return Property.toText(this.name, this.value);
  }

  static arrayFromText(text) {
    return text
      .replace(/-\W*/, " -")
      .replace(/\+\W*/g, " +")
      .split(/ /)
      .filter((i) => i.length)
      .map((t) => Property.fromText(t));
  }
}
