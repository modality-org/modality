export default class RoundRobin {
  constructor() {}

  static create() {
    return new RoundRobin();
  }

  async pickOne({ options, input }) {
    // absolute first round is round 1
    const i = parseInt(JSON.parse(input).round - 1) % options.length;
    return options[i];
  }
}
