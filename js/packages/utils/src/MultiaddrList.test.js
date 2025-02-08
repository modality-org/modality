import { expect, describe, test, it } from "@jest/globals";

import { resolveDnsEntries } from "./MultiaddrList";

describe("bootstrap", () => {
  it("should work", async () => {
    let r;
    r = await resolveDnsEntries([
      "/dns/example.com/tcp/80/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd",
    ]);
    expect(r[0]).toMatch(/^\/ip4\//);

    r = await resolveDnsEntries(["/dnsaddr/devnet3.modality.network"]);
    expect(r.length).toBe(3);
    expect(r[0]).toMatch(/^\/ip4\//);

    r = await resolveDnsEntries([
      "/dnsaddr/devnet3.modality.network/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd",
    ]);
    expect(r.length).toBe(1);
    expect(r[0]).toMatch(
      /\/p2p\/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd$/
    );
  });
});
