import Arrow from "./Arrow.js";

export default class State {
  constructor(id, arrows = []) {
    this.id = id;
    this.arrows = arrows;
    return this;
  }

  static createLooper(id = "anything") {
    const state = new State(id);
    state.arrows.push(new Arrow("", id));
    return state;
  }

  clone() {
    const s = new State();
    s.id = this.id;
    s.arrows = this.arrows.map((a) => a.clone());
    return s;
  }

  getArrow(index) {
    return Array.from(this.arrows)[index];
  }

  getPossibleNextStateIdsFor(properties_text) {
    const r = [];
    for (const arrow of this.arrows) {
      const ans = arrow.accepts(properties_text);
      if (ans.ok) {
        r.push(arrow.target_state_id);
      }
    }
    return r;
  }
}
