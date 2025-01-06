import State from "./State.js";
import Arrow from "./Arrow.js";

export default class System {
  constructor() {
    this.states_by_id = {};
    this.possible_current_state_ids = [];
    return this;
  }

  static createLooper() {
    const sys = new System();
    const s = State.createLooper();
    sys.states_by_id[s.id] = s;
    sys.possible_current_state_ids.push(s.id);
    return sys;
  }

  static fromJSON(json) {
    const state_info_by_id = json.states || {};
    const sys = new System();
    // create state for each key
    for (const state_id of Object.keys(state_info_by_id)) {
      const s = new State(state_id);
      sys.states_by_id[state_id] = s;
    }
    // add arrows to each state
    for (const state_id of Object.keys(state_info_by_id)) {
      for (const arrow of state_info_by_id[state_id].arrows) {
        const a = new Arrow(arrow[0], arrow[1]);
        sys.states_by_id[state_id].arrows.push(a);
      }
    }
    // set current states
    sys.possible_current_state_ids = json.possible_current_states;
    return sys;
  }

  toJSON() {
    const r = {
      states: {},
      possible_current_states: this.possible_current_state_ids,
    };
    for (const state of Object.values(this.states_by_id)) {
      const arrows = this.states_by_id[state.id].arrows;
      r.states[state.id] = {
        arrows: arrows.map((a) => a.toJSONArray()),
      };
    }
    return r;
  }

  clone() {
    const s = new System();
    s.states_by_id = Object.fromEntries(
      Object.entries(this.states_by_id).map((e) => [e[0], e[1].clone()])
    );
    s.possible_current_state_ids = [...this.possible_current_state_ids];
    return s;
  }

  getState(id) {
    return this.states_by_id[id];
  }

  getStates(ids) {
    return ids.map((id) => this.states_by_id[id]);
  }

  getAllStateIds() {
    return Object.keys(this.states_by_id);
  }

  getAllStates() {
    return this.getStates(this.getAllStateIds());
  }

  getPossibleCurrentStates() {
    return this.possible_current_state_ids.map((id) => this.states_by_id[id]);
  }

  getPossibleArrows() {
    const pcs = this.getPossibleCurrentStates();
    const r = [];
    for (const s of pcs) {
      s.arrows.forEach((a) => {
        r.push(a);
      });
    }
    return r;
  }

  getObservedProperties() {
    const r = new Set();
    const states = this.getAllStates();
    for (const s of states) {
      for (const a of s.arrows) {
        const props = a.getObservedProperties();
        for (const p of props) {
          r.add(p);
        }
      }
    }
    return r;
  }

  getPossibleNextStateIdsFor(properties_text) {
    const next_ids = new Set();
    const possible_current_states = this.getPossibleCurrentStates();
    for (const state of possible_current_states) {
      state.getPossibleNextStateIdsFor(properties_text).forEach((id) => {
        next_ids.add(id);
      });
    }
    return Array.from(next_ids);
  }

  getPossibleNextStatesFor(properties_text) {
    return this.getStates(this.getPossibleNextStateIdsFor(properties_text));
  }

  canTakeStep(step) {
    const state_ids = this.getPossibleNextStateIdsFor(step.properties_text);
    return !!state_ids.length;
  }

  takeStep(step) {
    const state_ids = this.getPossibleNextStateIdsFor(step.properties_text);
    if (!state_ids.length) {
      throw new Error("unable to take step");
    }
    this.possible_current_state_ids = state_ids;
  }
}
