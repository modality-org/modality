import { expect, describe, it } from "@jest/globals";

import Contract from "./Contract.js";

describe("Contract", () => {
  it("should work", async () => {
    const signed_init = await Contract.generateGenesis();
    expect(signed_init.genesis.contract_id).toBeTruthy();
    expect(signed_init["genesis.signature"]).toBeTruthy();

    const c = await Contract.generate();
    await c.appendCommitFromJson({
      body: [{ method: "post", path: "/test.txt", value: "hello world" }],
      head: {},
    });
    let r;
    r = await c.canAppendCommitFromJson({
      body: [{ method: "post", path: "/test.bad", value: "hello world" }],
      head: {},
    });
    expect(r).toBe(false);
  });
});
