import {
  unionOfSets,
  intersectionOfSets,
  setMinus,
  areSetsEqual,
} from "@modality-dev/utils/sets";

import {
  TrueAtom,
  FalseAtom,
  BoundVar,
  AndFormula,
  OrFormula,
  NotFormula,
  PropsAtom,
  BoxFormula,
  DiamondFormula,
  LfpFormula,
  GfpFormula,
} from "./ModalFormula.js";

const MAX_RECURSION = process.env.MAX_RECURSION || Infinity;

export default class Solve {
  static inSystem(formula, system, env = {}) {
    const formula_type = formula.constructor;
    if (formula_type === TrueAtom) {
      return new Set(system.getAllStateIds());
    } else if (formula_type === FalseAtom) {
      return new Set();
    } else if (formula_type === OrFormula) {
      return unionOfSets(
        Solve.inSystem(formula.left, system, env),
        Solve.inSystem(formula.right, system, env)
      );
    } else if (formula_type === AndFormula) {
      return intersectionOfSets(
        Solve.inSystem(formula.left, system, env),
        Solve.inSystem(formula.right, system, env)
      );
    } else if (formula_type === NotFormula) {
      const inverse_states = Solve.inSystem(formula.formula, system, env);
      const all_states = system.getAllStateIds();
      return setMinus(all_states, inverse_states);
    } else if (formula_type === BoxFormula) {
      //  ([inner] outer) evaluated at node:
      //    if inner not available as arrow then true
      //    else is outer true at _all_ of the next nodes?
      const outer_set = Solve.inSystem(formula.outer, system, env);
      const inner = formula.inner;
      const r = new Set();
      for (const state of Object.values(system.states_by_id)) {
        // include state if for all targetStates:
        //   (transitionLabel === action => innerSet.contains(targetState))
        // = !!(...)
        // = !(transitionLabel === action && !innerSet.contains(targetState))
        // = (transitionLabel !== action || innerSet.contains(targetState))
        let shouldIncludeState = true;
        for (const arrow of state.arrows) {
          if (
            arrow.accepts(inner.toText()).ok &&
            !outer_set.has(arrow.target_state_id)
          ) {
            shouldIncludeState = false;
            break;
          }
        }
        if (shouldIncludeState) {
          r.add(state.id);
        }
      }
      return r;
    } else if (formula_type === DiamondFormula) {
      //  (<inner> outer) evaluated at node:
      //    if inner not available as arrow then false
      //    else is outer true at _any_ of the next nodes?
      const outer_set = Solve.inSystem(formula.outer, system, env);
      const inner = formula.inner;
      const r = new Set();
      for (const state of Object.values(system.states_by_id)) {
        let shouldIncludeState = false;
        for (const arrow of state.arrows) {
          if (
            arrow.accepts(inner.toText(), false).ok &&
            outer_set.has(arrow.target_state_id)
          ) {
            shouldIncludeState = true;
            break;
          }
        }
        if (shouldIncludeState) {
          r.add(state.id);
        }
      }
      return r;
    } else if (formula_type === BoundVar) {
      if (env[formula.name]) {
        return env[formula.name];
      } else {
        return new Set();
      }
    } else if (formula_type === PropsAtom) {
      // TODO
      return true;
    } else if (formula_type === LfpFormula) {
      env[formula.bound_var.name] = new Set();
      let s;
      let i = 0;
      do {
        i = i + 1;
        s = new Set(env[formula.bound_var.name]);
        env[formula.bound_var.name] = Solve.inSystem(
          formula.inner,
          system,
          env
        );
      } while (
        i < MAX_RECURSION &&
        !areSetsEqual(s, env[formula.bound_var.name])
      );
      return s;
    } else if (formula_type === GfpFormula) {
      env[formula.bound_var.name] = new Set(system.getAllStateIds());
      let s;
      let i = 0;
      do {
        i = i + 1;
        s = new Set(env[formula.bound_var.name]);
        env[formula.bound_var.name] = Solve.inSystem(
          formula.inner,
          system,
          env
        );
      } while (
        i < MAX_RECURSION &&
        !areSetsEqual(s, env[formula.bound_var.name])
      );
      return s;
    }
    return new Set();
  }
}
