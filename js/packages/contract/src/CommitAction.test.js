import { expect, describe, it } from "@jest/globals";

import CommitAction from "./CommitAction.js";

describe("CommitAction", () => {
  it("should work", async () => {
    new CommitAction({
      method: "post",
      path: "/",
      value: "hello",
    });
    expect(() => {
      new CommitAction({
        method: "other",
        path: "/",
        value: "hello",
      });
    }).toThrow();
  });
});
