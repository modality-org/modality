import { expect, describe, it } from "@jest/globals";

import Arrow from "./Arrow";

describe("Arrow", () => {
  it("should work", async () => {
    const arrow = new Arrow("sign-defraud", null);
    let r;
    r = arrow.accepts("sign");
    expect(r.ok).toBe(true);
    r = arrow.accepts("sign+defraud");
    expect(r.ok).toBe(false);
  });
});
