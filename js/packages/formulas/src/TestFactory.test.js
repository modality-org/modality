import { expect, describe, test } from "@jest/globals";

import TestFactory from "./TestFactory.js";

class IsZeroTestFactory extends TestFactory {
  constructor() {
    super();
    this.name = "is_zero";
  }

  static name = "is_zero";

  getEvaluateForArgs(args) {
    const key = args[0];
    return async (context) => {
      return context[key] === 0;
    };
  }

  getCorrelateForArgs(args) {
    const key = args[0];
    return async (other_tests) => {
      return [
        {
          test: TestFactory.propFromNameAndArgs("is_one", [key]),
          rules: [
            `always must not (${TestFactory.propFromNameAndArgs("is_zero", [
              key,
            ])} and ${TestFactory.propFromNameAndArgs("is_zero", [key])})`,
          ],
        },
      ];
    };
  }
}

describe("Modality", () => {
  test("test function registration", async () => {
    const is_zero_tf = new IsZeroTestFactory();
    const is_zero__a = is_zero_tf.getTestForArgs(["a"]);

    expect(await is_zero__a.evaluate({ a: 0 })).toBe(true);
    expect(await is_zero__a.evaluate({ a: 1 })).toBe(false);
  });
});
