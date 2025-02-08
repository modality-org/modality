import { expect, describe, test, it, afterEach } from "@jest/globals";

import Node from "../../../src/Node.js";
import BlockHeader from "@modality-dev/network-datastore/data/BlockHeader";

import { dirname } from "dirname-filename-esm";
const __dirname = dirname(import.meta);
const FIXTURES_COMMON = `${__dirname}/../../../../../fixtures-common`;

describe("reqres /round/block_headers", () => {
  let node1, node2;

  it("should work", async () => {
    node1 = await Node.fromConfigFilepath(
      `${FIXTURES_COMMON}/network-node-configs/devnet2/node1.json`,
      { storage_path: null }
    );
    node2 = await Node.fromConfigFilepath(
      `${FIXTURES_COMMON}/network-node-configs/devnet2/node2.json`,
      { storage_path: null }
    );
    await node1.setupAsServer();
    await node2.setupAsClient();
    await node2.addPeerMultiaddress(
      await node1.getPeerId(),
      node1.getListenerMultiaddress()
    );

    const node1_datastore = node1.getDatastore();

    const rbh = await BlockHeader.fromJSONObject({
      round_id: 1,
      peer_id: node1.peerid,
    });
    await rbh.save({ datastore: node1_datastore });

    try {
      const r = await node2.swarm.services.reqres.call(
        node1.getListenerMultiaddress(),
        "/data/round/block_headers",
        { round_id: 1 }
      );
      expect(r.data.round_block_headers[0].round_id).toBe(1);
      expect(r.data.round_block_headers[0].peer_id).toBe(node1.peerid);
      expect(r.ok).toBe(true);
    } finally {
      await node1.stop();
      await node2.stop();
    }
  });

  afterEach(async () => {
    await node1?.stop();
    await node2?.stop();
  });
});
