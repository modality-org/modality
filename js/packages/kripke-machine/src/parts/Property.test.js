import { expect, describe, it } from "@jest/globals";

import Property from "./Property";

describe("Property", () => {
  it("should work", async () => {
    const true_love = new Property("love", true);
    expect(true_love.toText()).toBe("+love");
    const also_true_love = Property.fromText("love");
    expect(also_true_love.toText()).toBe("+love");
    const not_love = Property.fromText("-love");
    expect(not_love.toText()).toBe("-love");
  });
});
