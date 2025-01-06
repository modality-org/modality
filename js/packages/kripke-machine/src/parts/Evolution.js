import System from "./System.js";
export default class Evolution {
  constructor() {
    this.systems = [];
    this.mappings = [];
    return this;
  }

  static fromJSON(json) {
    if (!json) return null;
    const evolution = new Evolution();
    for (const json_ss of json.systems) {
      evolution.systems.push(System.fromJSON(json_ss));
    }

    evolution.mappings = json.mappings;
    return evolution;
  }

  getMappingTargets(system, state) {
    return this.mappings
      .map((a) => {
        if (system === a.source.system && state === a.source.state) {
          return a.target;
        }
      })
      .filter((i) => i);
  }
}
