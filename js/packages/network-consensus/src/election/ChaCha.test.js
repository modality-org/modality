import { jest, expect, describe, test, it } from "@jest/globals";

import ChaCha from "./ChaCha";

describe("ChaCha", () => {
  it("should work", async () => {
    const rand = new ChaCha();
    let result = await rand.pickOne({ options: [1, 2, 3], input: "123" });
    expect(result).toBe(1);
  });
});
