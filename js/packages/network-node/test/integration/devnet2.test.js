import { jest, expect, describe, test, it, afterEach } from "@jest/globals";
import Node from '../../src/Node.js';

import { dirname } from 'dirname-filename-esm';
const __dirname = dirname(import.meta);
const FIXTURES_COMMON = `${__dirname}/../../../../fixtures-common`;

describe.skip("devnet2", () => {
  let node1, node2;
  it("should work", async () => {
    node1 = await Node.fromConfigFilepath(`${FIXTURES_COMMON}/network-node-configs/devnet2/node1.json`, {storage_path: null});
    node2 = await Node.fromConfigFilepath(`${FIXTURES_COMMON}/network-node-configs/devnet2/node2.json`, {storage_path: null});
    try {
      await node1.setupAsServer();
      const consensus1 = await node1.setupLocalConsensus();
      consensus1.disableWaiting();

      await node2.setupAsServer();
      const consensus2 = await node2.setupLocalConsensus();
      consensus2.disableWaiting();

      await new Promise(r => setTimeout(r, 2*1000)); // they need time to connect

      const mockListener = jest.fn();
      node2.swarm.services.pubsub.addEventListener("message", mockListener);

      await Promise.all([
        consensus1.runRound(),
        consensus2.runRound()
      ]);

      expect(mockListener).toHaveBeenCalled();
    } finally {
      await node1.stop();
      await node2.stop();
    }
  }, 5*1000);

  afterEach(async () => {
    await node1?.stop();
    await node2?.stop();
  })
});
