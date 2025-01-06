import { expect, describe, it } from "@jest/globals";

import PropertyTable from "./PropertyTable";

describe("PropertyTable", () => {
  it("should work", async () => {
    let pt;
    pt = PropertyTable.fromText("+sign-defraud");
    expect(pt.keys().length).toBe(2);
    expect(() => {
      PropertyTable.fromText("+sign-sign");
    }).toThrow();
    pt = PropertyTable.fromText("+sign-defraud", true);
    expect(pt.get("sign")).toBe(true);
    expect(pt.get("defraud")).toBe(false);
    expect(pt.get("missing")).toBe(true);
    pt = PropertyTable.fromText("+sign-defraud", false);
    expect(pt.get("sign")).toBe(true);
    expect(pt.get("defraud")).toBe(false);
    expect(pt.get("missing")).toBe(false);
  });
});
