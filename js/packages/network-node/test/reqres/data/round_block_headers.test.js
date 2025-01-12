import { expect, describe, test, it } from "@jest/globals";

import Node from '../../../src/Node.js';
import RoundBlockHeader from "@modality-dev/network-datastore/data/RoundBlockHeader";

import { dirname } from 'dirname-filename-esm';
const __dirname = dirname(import.meta);
const FIXTURES_COMMON = `${__dirname}/../../../../../fixtures-common`;

describe("reqres /round_block_headers", () => {
  it("should work", async () => {
    const node1 = await Node.fromConfigFilepath(`${FIXTURES_COMMON}/network-node-configs/devnet2/node1.json`);
    const node2 = await Node.fromConfigFilepath(`${FIXTURES_COMMON}/network-node-configs/devnet2/node2.json`);
    await node1.setupAsServer();
    await node2.setupAsClient();
    await node2.addPeerMultiaddress(await node1.getPeerId(), node1.getListenerMultiaddress());

    const node1_datastore = node1.getDatastore();

    const rbh = await RoundBlockHeader.fromJSONObject({
      round_id: 1,
      peer_id: node1.peerid
    });
    await rbh.save({datastore: node1_datastore});

    try {
      const r = await node2.swarm.services.reqres.call(
        node1.getListenerMultiaddress(),
        "/data/round_block_headers",
        {round_id: 1}
      );
      expect(r.data.round_block_headers[0].round_id).toBe(1);
      expect(r.data.round_block_headers[0].peer_id).toBe(node1.peerid);
      expect(r.ok).toBe(true);
    } finally {
      await node1.stop();
      await node2.stop();
    }
  });
});
