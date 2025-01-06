import { jest, expect, describe, test, it } from "@jest/globals";
import * as HashTax from "./HashTax";

describe("HashTax", () => {
  it("should work", async () => {
    const data = "data";
    const difficulty = 500;
    const nonce = await HashTax.mine({
      data,
      difficulty,
    });
    expect(nonce).toBeTruthy();
    expect(nonce).toBe(2401);
    const validated = await HashTax.validateNonce({
      data,
      difficulty,
      nonce,
    });
    expect(validated).toBe(true);
  });
});
