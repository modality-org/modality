export default class StaticChoice {
  constructor(index = 0) {
    this.index = index;
  }

  async pickOne({ options }) {
    return options[this.index % options.length];
  }
}
