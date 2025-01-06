import System from "./parts/System.js";
import Step from "./parts/Step.js";
import Rule from "./parts/Rule.js";
import Solve from "./formulas/Solve.js";
import ModalMu from "./ModalMu.js";
import Evolution from "./parts/Evolution.js";
import Synthesizer from "@modality-dev/synthesizer";

import { isSupersetOf, intersectionOfSets } from "@modality-dev/utils/sets";

export default class KripkeMachine {
  constructor() {
    this.systems = [];
    this.rules = [];
    this.language = KripkeMachine.getDefaultLanguage;
  }

  static setDefaultLanguage(language) {
    this.languages = {
      ...(this.languages || {}),
      [language.name]: language,
    };
    this.default_language = language;
  }

  static getDefaultLanguage() {
    return this.default_language || ModalMu;
  }

  static setLanguage(name, language) {
    this.languages = {
      ...(this.languages || {}),
      [name]: language,
    };
  }

  static getLanguage(name) {
    this.languages?.[name];
  }

  static createLooper() {
    const km = new KripkeMachine();
    km.systems = [System.createLooper()];
    return km;
  }

  static fromJSON(json) {
    const km = new KripkeMachine();
    if (json.language) {
      km.language =
        KripkeMachine.getLanguage(json.language) ||
        KripkeMachine.getDefaultLanguage();
    }
    for (const systems_json of json.systems) {
      const sys = System.fromJSON(systems_json);
      km.systems.push(sys);
    }
    for (const rules_json of json.rules) {
      const rule = new Rule(rules_json.modal_formula, rules_json.root_states);
      km.rules.push(rule);
    }
    return km;
  }

  toJSON() {
    return {
      language: this.language.name,
      systems: this.systems.map((sys) => sys.toJSON()),
      rules: this.rules.map((rule) => rule.toJSON()),
    };
  }

  clone() {
    const km = new KripkeMachine();
    km.language = this.language;
    km.systems = this.systems.map((sys) => sys.clone());
    km.rules = this.rules.map((r) => r.clone());
    return km;
  }

  getPossibleCurrentStateIds() {
    const r = [];
    for (const [system_id, system] of this.systems.entries()) {
      for (const pcs of system.possible_current_state_ids) {
        r.push({ system: system_id, state: pcs });
      }
    }
    return r;
  }

  canTakeStep(step) {
    const km = this.clone();
    try {
      km.takeStep(step);
    } catch (e) {
      return [false, e.message];
    }
    return [true];
  }

  takeStep(step) {
    const km = this.clone();
    // Early Evolution
    if (step.hasEarlyEvolution()) {
      const evolution = step.getEvolution();
      if (!km.canEvolve(evolution, null)[0]) {
        throw new Error("early evolution failed");
      }
      km.evolve(evolution, null);
    }
    // Take step
    for (const sys of km.systems) {
      const r = sys.canTakeStep(step);
      if (!r) {
        console.error(
          [
            `no arrow matching step:`,
            step.properties_text,
            `\n\npossible arrows:`,
            ...sys.getPossibleArrows().map((a) => a.getProperties()),
          ].join("\n")
        );
        throw new Error(`can't take step`);
      }
    }
    for (const sys of km.systems) {
      sys.takeStep(step);
    }
    // No Evolution
    if (!step.hasEvolution()) {
      // No rule
      if (!step.hasRule()) {
        this.systems = km.systems;
        return;
      } else {
        // Rule without evolution
        const new_rule = new Rule(
          step.rule_text,
          km.getPossibleCurrentStateIds()
        );
        if (!km.satisfiesRule(new_rule)) {
          // Unsatisfiable new rule
          // Synthesize evolution
          const synth = new Synthesizer("simple");
          const synthesized_evolution_json = synth.getPossibleEvolutionJson(
            km.toJSON(),
            new_rule
          );
          const synthesized_evolution = Evolution.fromJSON(
            synthesized_evolution_json
          );
          if (!synthesized_evolution) {
            throw new Error(
              "Synthesizer could not produce a valid evolution for unsatisfiable rule"
            );
          }
          const [b, m] = km.canEvolve(synthesized_evolution, step.rule_text);
          if (!b) {
            throw new Error(
              "Synthesizer attempted to synthesize an evolution but failed to satisfy all the rules. " +
                m
            );
          }
          this.evolve(synthesized_evolution, step.rule_text);
          return;
        } else {
          // Satisfiable new rule
          this.rules.push(new_rule);
          this.systems = km.systems;
          return;
        }
      }
    } else if (!step.hasEarlyEvolution()) {
      // Late Evolution
      const evolution = step.getEvolution();
      if (!km.canEvolve(evolution, step.rule_text)[0]) {
        throw new Error("evolution failed");
      }
      this.evolve(evolution, step.rule_text);
    }
  }

  applyEvolution(evolution) {
    // move possible current states of each system
    const target_possible_current_states = [];
    for (const [source_ss_index, source_ss] of this.systems.entries()) {
      const source_all_pcs = source_ss.possible_current_state_ids;
      for (const pcs of source_all_pcs) {
        for (const target_pcs of evolution.getMappingTargets(
          source_ss_index,
          pcs
        )) {
          target_possible_current_states.push(target_pcs);
        }
      }
    }
    for (const [index, ess] of evolution.systems.entries()) {
      const pcs = target_possible_current_states
        .filter((i) => i.system === index)
        .map((i) => i.state);
      ess.possible_current_state_ids = pcs;
    }
    this.systems = evolution.systems;

    // move root states of rules
    for (const rule of this.rules) {
      const new_root_states = [];
      for (const root_state of rule.root_states) {
        const target_states = evolution.getMappingTargets(
          root_state.system,
          root_state.state
        );
        for (const target_state of target_states) {
          new_root_states.push(target_state);
        }
      }
      if (!new_root_states.length) {
        throw new Error("rule missing new root state");
      }
      rule.root_states = new_root_states;
    }
  }

  canEvolve(evolution, rule_text) {
    const km = this.clone();
    try {
      km.evolve(evolution, rule_text);
      return [true, null];
    } catch (e) {
      return [false, e];
    }
  }

  evolve(evolution, rule_text) {
    const km = this.clone();
    if (rule_text) {
      const new_rule = new Rule(rule_text, km.getPossibleCurrentStateIds());
      km.rules.push(new_rule);
    }
    if (evolution) {
      km.applyEvolution(evolution);
    }
    const r = km.satisfiesRules();
    if (!r.ok) {
      throw new Error(
        `new rule not satisfied: \n${r.errors
          .map((i) => i.modal_formula_text)
          .join(", ")}`
      );
    }
    this.rules = km.rules;
    this.systems = km.systems;
  }

  satisfiesRules() {
    for (const rule of this.rules) {
      if (!this.satisfiesRule(rule)) {
        return { ok: false, errors: [rule] };
      }
    }
    return { ok: true };
  }

  satisfiesRule(rule) {
    const rule_props = rule.getProps();
    let skipped_all = true;
    for (const [system_index, system] of this.systems.entries()) {
      const r = Solve.inSystem(rule.modal_formula, system);
      const system_root_states = rule.root_states
        .filter((i) => i.system === system_index)
        .map((i) => i.state);
      const ok = isSupersetOf(r, new Set(system_root_states));
      if (!ok) {
        const system_observed_props = system.getObservedProperties();
        const rel_props = intersectionOfSets(system_observed_props, rule_props);
        if (rel_props.size === 0) {
          continue;
        } else {
          return false;
        }
      } else {
        skipped_all = false;
      }
    }
    if (skipped_all) {
      return false;
    }
    return true;
  }
}
