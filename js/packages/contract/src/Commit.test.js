import { expect, describe, it } from "@jest/globals";

import Commit from "./Commit.js";

describe("Contract", () => {
  it("should work", async () => {
    Commit.fromJSON({
      body: [{ method: "post", path: "/value.text", value: "hello" }],
      head: [],
    });
    expect(() => {
      Commit.fromJSON({
        body: { method: "post", path: "/value.text", value: "hello" },
        head: [],
      });
    }).toThrow();
    expect(() => {
      Commit.fromJSON({
        body: [{ method: "wrong", path: "/value.text", value: "hello" }],
        head: [],
      });
    }).toThrow();

    const c = new Commit();
    c.addPost("/value.text", "hello");
    c.addPost("/value2.text", "hello");
    expect(c.body[0].path).toBe("/value.text");
    expect(c.body.length).toBe(2);
    expect(() => {
      c.addPost("/value.text", "hello again");
    }).toThrow();
    c.addRule("true");
    expect(() => {
      c.addRule("???");
    }).toThrow();
  });
});
