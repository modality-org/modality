import { expect, describe, test, it } from "@jest/globals";

import Node from '../src/Node.js';

import { dirname } from 'dirname-filename-esm';
const __dirname = dirname(import.meta);
const FIXTURES_COMMON = `${__dirname}/../../../fixtures-common`;

describe("sync blocks", () => {
  it("should work", async () => {
    const node1 = await Node.fromConfigFilepath(`${FIXTURES_COMMON}/network-node-configs/devnet1/node1.json`, {storage_path: null});
    const node2 = await Node.createNetworkClient('devnet1');
    await node1.setupAsServer();
    await node2.setupAsClient();

    const consensus1 = await node1.setupLocalConsensus();
    consensus1.disableWaiting();
    await node1.getDatastore().setCurrentRound(0);
    for (let i = 1; i <= 10; i++) {
      await consensus1.runRound();
    }

    try {
      let r;
      r = await node2.sendRequest(node1.getListenerMultiaddress(), "/status");
      expect(r.ok).toBe(true);
      expect(r.data).toStrictEqual({"current_round": 11});

      r = await node2.sendRequest(node1.getListenerMultiaddress(), "/data/round_block_headers", {round_id: 0});
      expect(r.data.round_block_headers.length).toBe(1)

      r = await node2.sendRequest(node1.getListenerMultiaddress(), "/data/round_block_headers", {round_id: 5});
      expect(r.data.round_block_headers.length).toBe(1)

      r = await node2.sendRequest(node1.getListenerMultiaddress(), "/data/round_block_headers", {round_id: 10});
      expect(r.data.round_block_headers.length).toBe(1)

    } finally {
      await node1.stop();
      await node2.stop();
    }
  });
});
