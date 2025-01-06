import { toString as uint8ArrayToString } from "uint8arrays/to-string";
import { fromString as uint8ArrayFromString } from "uint8arrays/from-string";

import Test from "./Test.js";

export function escapeArgs(...args) {
  const arg_suffix = uint8ArrayFromString(JSON.stringify(args || [])).join("_");
  return `${arg_suffix}`;
}

export function unescapeArgs(args) {
  const arg_bytes = args.split("_").map((i) => parseInt(i));
  const str = uint8ArrayToString(new Uint8Array(arg_bytes));
  return str;
}

export function getPropFromTest(name, args) {
  const arg_suffix = uint8ArrayFromString(JSON.stringify(args || [])).join("_");
  return `${name}__${arg_suffix}`;
}

export function getNameFromProp(prop) {
  const m = prop.match(/^([a-zA-Z][a-zA-Z0-9_]*)__([0-9_]*)$/);
  if (!m) {
    return null;
  }
  return m[1];
}

export function parseProp(prop) {
  const m = prop.match(/^([a-zA-Z][a-zA-Z0-9_]*)__([0-9_]*)$/);
  if (!m) {
    return null;
  }
  const name = m[1];
  const arg_suffix = m[2];
  const arg_bytes = arg_suffix.split("_").map((i) => parseInt(i));
  const args_string = uint8ArrayToString(new Uint8Array(arg_bytes));
  const args = JSON.parse(args_string);
  return {
    name,
    args,
  };
}

export default class TestFactory {
  constructor(name) {
    this.name = name;
    return this;
  }

  static escapeArgs = escapeArgs;
  static unescapeArgs = unescapeArgs;
  static getPropFromTest = getPropFromTest;
  static getNameFromProp = getNameFromProp;
  static parseProp = parseProp;
  static propFromNameAndArgs = getPropFromTest;

  getArgsFromProp(prop) {
    const call = parseProp(prop);
    if (call.name === this.name) {
      return call.args;
    }
  }

  getPropWithArgs(args) {
    return getPropFromTest(this.name, args);
  }

  getTestForArgs(args) {
    const test = new Test(this.name, args);
    test.evaluate = this.getEvaluateForArgs(args);
    test.correlate = this.getCorrelateForArgs(args);
    return test;
  }

  getEvaluateForArgs(args) {
    throw new Error("not implemented");
  }

  getCorrelateForArgs(args) {
    throw new Error("not implemented");
  }
}
