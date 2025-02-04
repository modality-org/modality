import { expect, describe, test, it, afterEach } from "@jest/globals";

// import createTestNode from "../../createTestNode";
import Node from '../../src/Node.js';

import { dirname } from 'dirname-filename-esm';
const __dirname = dirname(import.meta);
const FIXTURES_COMMON = `${__dirname}/../../../../fixtures-common`;

describe("reqres /status", () => {
  let node1, node2;
  it("should work", async () => {
    const node1 = await Node.fromConfigFilepath(`${FIXTURES_COMMON}/network-node-configs/devnet2/node1.json`, {storage_path: null});
    const node2 = await Node.fromConfigFilepath(`${FIXTURES_COMMON}/network-node-configs/devnet2/node2.json`, {storage_path: null});
    await node1.setupAsClient();
    await node2.setupAsServer();

    await node1.swarm.peerStore.save(
      await node2.getPeerId(),
      {multiaddrs: [node2.getListenerMultiaddress()]}
    );

    await node2.getDatastore().setCurrentRound(1);

    try {
      const r = await node1.sendRequest(
        node2.getListenerMultiaddress(),
        "/status",
      );
      expect(r.ok).toBe(true);
      expect(r.data).toStrictEqual({"current_round": 1});
    } finally {
      await node1.stop();
      await node2.stop();
    }
  });

  afterEach(async () => {
    await node1?.stop();
    await node2?.stop();
  })
});
