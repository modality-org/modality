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
    new CommitAction({
      method: "repost",
      path: "/reposts/src/notes/hello.text",
      value: "hi",
      source_contract: "src",
      source_path: "/notes/hello.text",
      source_commit: "abc",
    }).validateOrThrow();
    expect(() => {
      new CommitAction({
        method: "repost",
        path: "/reposts/src/notes/hello.text",
        value: "hi",
      }).validateOrThrow();
    }).toThrow(/source_contract/);
  });
});
