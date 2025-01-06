import Evolution from "./Evolution.js";

export default class Step {
  constructor(properties_text, opts = {}) {
    this.properties_text = properties_text;
    this.rule_text = opts.rule_text;
    this.evolution_json = opts.evolution_json;
    return this;
  }

  hasRule() {
    return !!this.rule_text;
  }

  hasEvolution() {
    return !!this.evolution_json;
  }

  getEvolution() {
    if (!this.evolution_json) {
      return null;
    }
    return Evolution.fromJSON(this.evolution_json);
  }

  hasEarlyEvolution() {
    if (!this.evolution_json) {
      return null;
    }
    return this.evolution_json.apply_first;
  }
}
