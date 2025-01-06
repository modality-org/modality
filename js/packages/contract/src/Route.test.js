import { expect, describe, it } from "@jest/globals";

import Route from "./Route.js";

describe("Route", () => {
  it("should work", async () => {
    expect(Route.getType("/test.txt")).toBe("attachment");
    expect(Route.getType("/test.text")).toBe("primitive");
    expect(Route.getType("not/")).toBe(null);
    expect(Route.getType("/not.text/test.txt")).toBe(null);
    expect(Route.getPathParts("/nested/thing/test.txt")).toEqual({
      folder_path: "/nested/thing/",
      file_name: "test",
      file_type: "txt",
    });
  });
});
