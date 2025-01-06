import { expect, describe, it } from "@jest/globals";

import KripkeMachine from "../KripkeMachine.js";
import Step from "../parts/Step.js";
import Evolution from "../parts/Evolution.js";

import JSONFile from "@modality-dev/utils/JSONFile";
import { dirname } from 'dirname-filename-esm';
const __dirname = dirname(import.meta);
function fixturesFile(fn) {
  return `${__dirname}/../../fixtures/${fn}`;
}

describe("KripkeMachine", () => {
  it("early evolves", async () => {
    let km, r, step;

    const json = JSONFile.readSync(fixturesFile("km/looper.json"));
    km = KripkeMachine.fromJSON(json);

    const evolution_one_party_json = JSONFile.readSync(
      fixturesFile("evolution/one_party.json")
    );
    const evolution_one_party = Evolution.fromJSON(evolution_one_party_json);

    const km1 = km.clone();
    [r] = km1.canEvolve(
      evolution_one_party,
      "gfp(@x, []@x and ([-PARTY_A_SIGNS]false or [-PARTY_B_SIGNS]false))"
    );
    expect(r).toBe(true);
    km1.evolve(
      evolution_one_party,
      "gfp(@x, []@x and ([-PARTY_A_SIGNS]false or [-PARTY_B_SIGNS]false))"
    );

    const evolution_either_party_json = JSONFile.readSync(
      fixturesFile("evolution/either_party.json")
    );
    const evolution_either_party = Evolution.fromJSON(
      evolution_either_party_json
    );
    const km2 = km1.clone();
    [r] = km2.canEvolve(evolution_either_party, null);
    expect(r).toBe(true);

    const km3 = km1.clone();
    step = new Step("+PARTY_B_SIGNS", {
      evolution_json: { ...evolution_either_party_json, apply_first: true },
    });
    [r] = km3.canTakeStep(step);
    expect(r).toBe(true);
  });
});
