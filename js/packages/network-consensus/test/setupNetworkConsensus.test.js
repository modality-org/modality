import { jest, expect, describe, test, it } from "@jest/globals";
import NetworkDatastore from "@modality-dev/network-datastore";
import Devnet from "@modality-dev/network-configs/Devnet";

import { setupNetworkConsensus } from "../src";

describe("setupNetworkConsensus", () => {
  it("should work", async () => {
    const datastore = await NetworkDatastore.createInMemory();
    const keypair = await Devnet.getKeypairByIndex(0);
    const nc = await setupNetworkConsensus({
      datastore,
      keypair,
      peerid: keypair.id,
      sequencing_method: 'DAGRider',
      election_method: 'RoundRobin'
    });
    expect(nc).not.toBeNull();
  });
});
