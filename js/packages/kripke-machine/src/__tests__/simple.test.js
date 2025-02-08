import { expect, describe, it } from "@jest/globals";

import KripkeMachine from "../KripkeMachine.js";
import Step from "../parts/Step.js";
import Evolution from "../parts/Evolution.js";

import JSONFile from "@modality-dev/utils/JSONFile";
import { dirname } from "dirname-filename-esm";
const __dirname = dirname(import.meta);
function fixturesFile(fn) {
  return `${__dirname}/../../fixtures/${fn}`;
}

describe("KripkeMachine", () => {
  it("should work", async () => {
    let km, r, e;
    km = KripkeMachine.createLooper();
    [r] = km.canTakeStep(new Step("anything"));
    expect(r).toBe(true);

    const json = JSONFile.readSync(fixturesFile("km/two_step.json"));
    km = KripkeMachine.fromJSON(json);

    [r] = km.canTakeStep(new Step("left"));
    expect(r).toBe(true);
    [r] = km.canTakeStep(new Step("right"));
    expect(r).toBe(true);

    km.takeStep(new Step("left"));
    [r] = km.canTakeStep(new Step("left"));
    expect(r).toBe(false);
    [r] = km.canTakeStep(new Step("right"));
    expect(r).toBe(true);
  });

  it("evolves", async () => {
    let km, r, e;
    const json = JSONFile.readSync(fixturesFile("km/sign.json"));
    km = KripkeMachine.fromJSON(json);

    const evolution_json = JSONFile.readSync(
      fixturesFile("evolution/sign_and_not_defraud.json")
    );
    const evolution = Evolution.fromJSON(evolution_json);
    [r, e] = km.canEvolve(evolution);
    expect(r).toBe(true);

    // test trivial rules with evolution
    const km1 = km.clone();
    [r, e] = km1.canEvolve(null, "true");
    expect(r).toBe(true);
    [r] = km1.canEvolve(null, "false");
    expect(r).toBe(false);

    // test rule and its inverse rule with evolution
    const km2 = km.clone();
    [r] = km2.canEvolve(null, "gfp(@x, []@x and [defraud]false)");
    expect(r).toBe(true);

    const km2a = km.clone();
    km2a.evolve(evolution);
    [r, e] = km2a.canEvolve(null, "gfp(@x, []@x and [-defraud]false)");
    expect(r).toBe(false);

    // test application of evolution without rule
    const km3 = km.clone();
    km3.applyEvolution(evolution);
    expect(km3.rules.length).toBe(1);
    expect(km3.systems.length).toBe(2);

    // test evolution with unsatisfiable rule and satisfiable rule
    const km4 = km.clone();
    expect(() => {
      km4.evolve(evolution, "lfp(@x, []@x and <defraud>true)");
    }).toThrow();
    km4.evolve(evolution, "gfp(@x, []@x and [defraud]false)");
    expect(km4.rules.length).toBe(2);
    expect(km4.systems.length).toBe(2);
  });
});
