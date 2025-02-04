import { jest, expect, describe, test, it } from "@jest/globals";

import NetworkDatastore from "./NetworkDatastore.js";

describe("NetworkDatastore", () => {
  it("should work", async () => {
    let v;
    const datastore = await NetworkDatastore.createInMemory();
    await datastore.put("/blocks/1", "");
    await datastore.put("/blocks/2", "");
    await datastore.put("/blocks/3", "");
    await datastore.put("/blocks/4", "");
    await datastore.put("/blocks/30", "");
    v = await datastore.findMaxStringKey("/blocks");
    expect(v).toBe("4");
    v = await datastore.findMaxIntKey("/blocks");
    expect(v).toBe(30);
    const it = await datastore.iterator({ prefix: "" });

    let key_count = 0;
    for await (const [key, value] of it) {
      key_count++;
    }
    expect(key_count).toBe(5);
  });
});
