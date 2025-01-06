const log = console.log;

export default class Synthesizer {
  constructor(mode = "simple") {
    this.mode = mode;
  }

  /*
    Simple Heuristics:
    Heristic 1: Always - Must
    If new rule is of the form - 
        always (must(x1)) and always (must(x2)) and ... and always (must(xn)) where x1, x2, ..., xn are individual propositions
        then, for each system, all arrows must satisfy [-x1 -x2 ... -xn]
    */
  check_always_must_heuristic(modal_formula) {
    if (modal_formula.constructor.name === "GfpFormula") {
      const gfp_inner = modal_formula.inner.right;
      if (gfp_inner.constructor.name === "BoxFormula") {
        const box_inner = gfp_inner.inner;
        if (box_inner.constructor.name === "PropsAtom") {
          return true;
        }
      }
    } else if (modal_formula.constructor.name === "AndFormula") {
      if (
        modal_formula.left.constructor.name === "AndFormula" ||
        modal_formula.right.constructor.name === "GfpFormula"
      ) {
        return (
          this.check_always_must_heuristic(modal_formula.left) &&
          this.check_always_must_heuristic(modal_formula.right)
        );
      }
    }

    return false;
  }

  get_always_must_evolution(evolution_json, new_rule) {
    const signed_props = new_rule.modal_formula.getSignedProps();
    for (const signed_prop of signed_props) {
      const unsigned_prop = signed_prop.slice(1);
      const corr_sign = signed_prop[0] === "-" ? "+" : "-";
      for (const system of evolution_json.systems) {
        for (const state_name of Object.keys(system.states)) {
          let new_arrows = [];
          for (const [arr_indx, arrow] of system.states[
            state_name
          ].arrows.entries()) {
            const index = arrow[0].indexOf(unsigned_prop);
            if (index != -1) {
              if (signed_prop[0] === corr_sign) new_arrows.push(arrow);
            } else {
              new_arrows.push(arrow);
              new_arrows[new_arrows.length - 1][0] +=
                " " + corr_sign + unsigned_prop + " ";
            }
          }
          system.states[state_name].arrows = new_arrows;
        }
      }
    }

    return evolution_json;
  }

  getPossibleEvolutionJson(km_json, new_rule) {
    let evolution_json = {};
    const modal_formula = new_rule.modal_formula;

    if (this.mode === "simple") {
      // No change in rules
      evolution_json.rules = km_json.rules;

      // No change in states
      evolution_json.mappings = [];

      for (const [sys_index, system] of km_json.systems.entries()) {
        for (const state_name of Object.keys(system.states)) {
          evolution_json.mappings.push({
            source: { system: sys_index, state: state_name },
            target: { system: sys_index, state: state_name },
          });
        }
      }

      evolution_json.systems = km_json.systems;

      // Run Simple heuristics
      if (this.check_always_must_heuristic(modal_formula)) {
        evolution_json = this.get_always_must_evolution(
          evolution_json,
          new_rule
        );
        return evolution_json;
      }
    }

    return null;
  }
}
