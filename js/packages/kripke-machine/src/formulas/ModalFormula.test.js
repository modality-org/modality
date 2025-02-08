import { expect, describe, it } from "@jest/globals";

import ModalFormula from "./ModalFormula.js";
import Solve from "./Solve.js";
import System from "../parts/System.js";

import JSONFile from "@modality-dev/utils/JSONFile";
import { dirname } from "dirname-filename-esm";
const __dirname = dirname(import.meta);
function fixturesFile(fn) {
  return `${__dirname}/../../fixtures/${fn}`;
}

describe("ModalFormula", () => {
  let f;

  it("should parse booleans", async () => {
    new ModalFormula("true");
    new ModalFormula("false");
    expect(() => {
      new ModalFormula("true]");
    }).toThrow();
    f = new ModalFormula("not(false)");
    expect(await f.getValue()).toBe(true);
    f = new ModalFormula("not(true)");
    expect(await f.getValue()).toBe(false);
    f = new ModalFormula("true and (true or false)");
    expect(await f.getValue()).toBe(true);
    f = new ModalFormula("false or (true and false)");
    expect(await f.getValue()).toBe(false);
  });

  it("should parse multiActions", async () => {
    new ModalFormula("hello");
    new ModalFormula("+hello");
    new ModalFormula("-hello");
    new ModalFormula("hello + true - false");
    new ModalFormula("- hello + true - false");
    new ModalFormula("+ hello + true - false");
    const text = "true and +hello +true -false";
    f = new ModalFormula(text);
    expect(f.toText()).toBe(text);
    expect(() => {
      new ModalFormula("- hello true + false");
    }).toThrow();
  });

  it("should parse HML style boxes and diamonds", async () => {
    new ModalFormula("[write] true");
    f = new ModalFormula("[write] [write] <publish> true");
  });

  it("should parse empty multiActions, boxes, and diamonds", async () => {
    new ModalFormula("");
    new ModalFormula("[] true");
    new ModalFormula("<> true");
  });

  it("should parse modal mu fixed points formula", async () => {
    new ModalFormula("lfp(@x, true)");
    const text = "gfp(@x, []@x and [-write] false)";
    f = new ModalFormula(text);
    expect(f.toText()).toBe("gfp(@x, [] @x and [-write] false)");
  });

  it("should be solvable in an system", async () => {
    let r;
    const looper_json = JSONFile.readSync(fixturesFile("km/looper.json"));
    const looper_ss = System.fromJSON(looper_json.systems[0]);

    const sign_json = JSONFile.readSync(fixturesFile("km/sign.json"));
    const sign_ss = System.fromJSON(sign_json.systems[0]);

    f = new ModalFormula("[write] true");
    r = Solve.inSystem(f, looper_ss);
    expect(r.size).toBe(1);
    expect(r.has("_")).toBe(true);

    f = new ModalFormula("[write] false");
    r = Solve.inSystem(f, looper_ss);
    expect(r.size).toBe(0);

    f = new ModalFormula("<write> true");
    r = Solve.inSystem(f, looper_ss);
    expect(r.size).toBe(1);
    expect(r.has("_")).toBe(true);

    f = new ModalFormula("[-sign] false");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(1);

    f = new ModalFormula("<-sign> true");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(0);

    f = new ModalFormula("gfp(@x, @x)");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(1);

    f = new ModalFormula("lfp(@x, @x)");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(0);

    f = new ModalFormula("gfp(@x, true)");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(1);

    f = new ModalFormula("lfp(@x, true)");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(1);

    f = new ModalFormula("gfp(@x, false)");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(0);

    f = new ModalFormula("lfp(@x, false)");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(0);

    f = new ModalFormula("gfp(@x, @x)");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(1);

    f = new ModalFormula("lfp(@x, @x)");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(0);

    f = new ModalFormula("gfp(@x, []@x)");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(1);

    f = new ModalFormula("lfp(@x, <>@x)");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(0);

    f = new ModalFormula("gfp(@x, [-sign]false)");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(1);

    f = new ModalFormula("gfp(@x, [sign]false)");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(0);

    f = new ModalFormula("lfp(@x, [-sign]false)");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(1);

    f = new ModalFormula("lfp(@x, [sign]false)");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(0);

    f = new ModalFormula("gfp(@x, []@x and [-sign] false)");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(1);

    f = new ModalFormula("gfp(@x, []@x and [sign] false)");
    r = Solve.inSystem(f, sign_ss);
    expect(r.size).toBe(0);
  });
});
