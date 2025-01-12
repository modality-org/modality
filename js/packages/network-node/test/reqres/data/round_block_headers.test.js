import { expect, describe, test, it } from "@jest/globals";

import Node from '../../../src/Node.js';

import { dirname } from 'dirname-filename-esm';
const __dirname = dirname(import.meta);
const FIXTURES_COMMON = `${__dirname}/../../../../../fixtures-common`;

describe("reqres /round_block_headers", () => {
  it("should work", async () => {
    const node1 = await Node.fromConfigFilepath(`${FIXTURES_COMMON}/network-node-configs/devnet2/node1.json`);
    const node2 = await Node.fromConfigFilepath(`${FIXTURES_COMMON}/network-node-configs/devnet2/node2.json`);
    await node1.setupAsClient();
    await node2.setupAsServer();

    await node1.swarm.peerStore.save(
      await node2.getPeerId(),
      {multiaddrs: [node2.getListenerMultiaddress()]}
    );

    try {
      const r = await node1.swarm.services.reqres.call(
        node2.getListenerMultiaddress(),
        "/data/round_block_headers",
        {round_id: 1}
      );
      expect(r.ok).toBe(true);
    } finally {
      await node1.stop();
      await node2.stop();
    }
  });
});
