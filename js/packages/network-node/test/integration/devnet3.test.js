import { jest, expect, describe, test, it, afterEach } from "@jest/globals";
import Node from '../../src/Node.js';

import { dirname } from 'dirname-filename-esm';
const __dirname = dirname(import.meta);
const FIXTURES_COMMON = `${__dirname}/../../../../fixtures-common`;

describe("devnet3", () => {
  let node1, node2, node3;
  it("should work", async () => {
    node1 = await Node.fromConfigFilepath(`${FIXTURES_COMMON}/network-node-configs/devnet3/node1.json`, {storage_path: null});
    node2 = await Node.fromConfigFilepath(`${FIXTURES_COMMON}/network-node-configs/devnet3/node2.json`, {storage_path: null});
    node3 = await Node.fromConfigFilepath(`${FIXTURES_COMMON}/network-node-configs/devnet3/node3.json`, {storage_path: null});
    try {
      await node1.setupAsServer();
      const consensus1 = await node1.setupLocalConsensus();
      consensus1.disableWaiting();

      await node2.setupAsServer();
      const consensus2 = await node2.setupLocalConsensus();
      consensus2.disableWaiting();

      await node3.setupAsServer();
      const consensus3 = await node3.setupLocalConsensus();
      consensus3.disableWaiting();

      await new Promise(r => setTimeout(r, 2*1000)); // they need time to connect

      const mockListener = jest.fn();
      node3.swarm.services.pubsub.addEventListener("message", mockListener);

      await Promise.all([
        consensus1.runRound(),
        consensus2.runRound(),
        consensus3.runRound()
      ]);

      expect(mockListener).toHaveBeenCalled();

      for (let i = 0; i < 10; i++) {
        await Promise.all([
          consensus1.runRound(),
          consensus2.runRound(),
          consensus3.runRound(),
        ]);
      }

      const round_id = await node2.getDatastore().getCurrentRound();
      expect(round_id).toBe(12);
    } finally {
      await node1.stop();
      await node2.stop();
      await node3.stop();
    }
  }, 15*1000);

  afterEach(async () => {
    await node1?.stop();
    await node2?.stop();
    await node3?.stop();
  })
});
