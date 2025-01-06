import { jest, expect, describe, test, it } from "@jest/globals";

import { setTimeout as setTimeoutPromise } from 'timers/promises';

import Devnet from "@modality-dev/network-configs/Devnet";

import TestNetwork from "../TestNetwork";

describe("Bullshark", () => {
  // to make testing easy to understand
  // round robin is used to elect leaders

  test("run sequencers", async () => {
    const NODE_COUNT = 9;
    const my_seq_id = Devnet.peeridOf(0);

    const network = await TestNetwork.setup({node_count: NODE_COUNT, sequencing_method: 'Bullshark', election_method: 'RoundRobin'});
    await network.runUntilRound(9);
    const runner1 = network.getNode(my_seq_id).runner;

    const leader1 = await runner1.sequencing.findLeaderInRound(1);
    expect(leader1).not.toBeNull();
    const leader5 = await runner1.sequencing.findLeaderInRound(5);
    expect(leader5).not.toBeNull();
    const pages = await runner1.sequencing.findOrderedPagesInSection(null, 5);
    expect(pages.length).toBe(NODE_COUNT * 4 + 1);
  });

  test("given f = 1, one bad sequencer not elected leader, network can sequence", async () => {
    const NODE_COUNT = 4;
    const BAD_NODE_COUNT = 1;
    const my_seq_id = Devnet.peeridOf(0);
    const offline_seq_id = Devnet.peeridOf(3);

    const network = await TestNetwork.setup({node_count: NODE_COUNT, sequencing_method: 'Bullshark', election_method: 'RoundRobin'});
    network.communication.offline_nodes = [offline_seq_id];
    await network.runUntilRound(9);

    const runner1 = network.getNode(my_seq_id).runner;
    const leader1 = await runner1.sequencing.findLeaderInRound(1);
    expect(leader1).not.toBeNull();
    const leader5 = await runner1.sequencing.findLeaderInRound(5);
    expect(leader5).not.toBeNull();
    const pages = await runner1.sequencing.findOrderedPagesInSection(null, 5);
    expect(pages.length).toBe((NODE_COUNT - BAD_NODE_COUNT) * 4 + 1 + BAD_NODE_COUNT);
  });

  test("given f = 0, one bad sequence, network stalls", async () => {
    const NODE_COUNT = 3;
    const BAD_NODE_COUNT = 1;
    const offline_seq_id = Devnet.peeridOf(NODE_COUNT - 1);

    const network = await TestNetwork.setup({node_count: NODE_COUNT, sequencing_method: 'Bullshark', election_method: 'RoundRobin'});
    network.communication.offline_nodes = [offline_seq_id];

    const abortController = new AbortController();
    setTimeoutPromise(3000).then(() => { abortController.abort() });    
    await expect(network.runUntilRound(9, abortController.signal)).rejects.toThrow("aborted");
  });
});
