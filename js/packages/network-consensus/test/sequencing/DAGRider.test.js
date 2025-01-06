import { jest, expect, describe, test, it } from "@jest/globals";

import { setTimeout as setTimeoutPromise } from 'timers/promises';

import NetworkDatastoreBuilder from "@modality-dev/network-datastore/NetworkDatastoreBuilder";

import Devnet from "@modality-dev/network-configs/Devnet";

import DAGRider from "../../src/sequencing/DAGRider";
import RoundRobin from "../../src/election/RoundRobin";
import ConsensusMath from "../../src/lib/ConsensusMath";

import TestNetwork from "../TestNetwork";

describe("DAGRider", () => {
  // to make testing easy to understand
  // round robin is used to elect leaders
  const election = RoundRobin.create();

  // when rounds are fully connected, pages a few rounds back can be sequenced
  // in particular,
  test("sequencing given fully connected rounds", async () => {
    const NODE_COUNT = 3;
    let pages, page, page1;

    // setup
    const scribes = await Devnet.getPeerids(NODE_COUNT);
    const scribe_keypairs = await Devnet.getKeypairsDict(NODE_COUNT);
    const ds_builder = await NetworkDatastoreBuilder.createInMemory();
    const sequencing = DAGRider.create({
      datastore: ds_builder.datastore,
      election
    });

    ds_builder.scribes = [...scribes];
    ds_builder.scribe_keypairs = scribe_keypairs;

    // round 1
    await ds_builder.addFullyConnectedRound();
    page1 = await sequencing.findLeaderInRound(1);
    expect(page1).toBeNull();

    // round 2
    await ds_builder.addFullyConnectedRound();
    page1 = await sequencing.findLeaderInRound(1);
    expect(page1).toBeNull();
    page = await sequencing.findLeaderInRound(2);
    expect(page).toBeNull();

    // round 3
    await ds_builder.addFullyConnectedRound();
    page1 = await sequencing.findLeaderInRound(1);
    expect(page1).toBeNull();
    page = await sequencing.findLeaderInRound(2);
    expect(page).toBeNull();
    page = await sequencing.findLeaderInRound(3);
    expect(page).toBeNull();

    // round 4
    await ds_builder.addFullyConnectedRound();
    page1 = await sequencing.findLeaderInRound(1);
    expect(page1).not.toBeNull();
    page = await sequencing.findLeaderInRound(2);
    expect(page).toBeNull();
    page = await sequencing.findLeaderInRound(3);
    expect(page).toBeNull();
    page = await sequencing.findLeaderInRound(4);
    expect(page).toBeNull();
    pages = await sequencing.findOrderedPagesInSection(null, 1);
    expect(pages.length).toBe(1); // first section is only one page
    expect(pages.at(-1).scribe).toBe(page1.scribe);

    // round 8
    await ds_builder.addFullyConnectedRound();
    await ds_builder.addFullyConnectedRound();
    await ds_builder.addFullyConnectedRound();
    await ds_builder.addFullyConnectedRound();
    pages = await sequencing.findOrderedPagesInSection(1, 5);
    expect(pages.length).toBe(4 * NODE_COUNT);
    expect(pages.at(-1).scribe).toBe(scribes[1]);

    // round 12
    await ds_builder.addFullyConnectedRound();
    await ds_builder.addFullyConnectedRound();
    await ds_builder.addFullyConnectedRound();
    await ds_builder.addFullyConnectedRound();
    pages = await sequencing.findOrderedPagesInSection(5, 9);
    expect(pages.length).toBe(4 * NODE_COUNT);
    expect(pages.at(-1).scribe).toBe(scribes[2]);

    // round 16
    await ds_builder.addFullyConnectedRound();
    await ds_builder.addFullyConnectedRound();
    await ds_builder.addFullyConnectedRound();
    await ds_builder.addFullyConnectedRound();
    pages = await sequencing.findOrderedPagesInSection(9, 13);
    expect(pages.length).toBe(4 * NODE_COUNT);
    expect(pages.at(-1).scribe).toBe(scribes[0]);

    let leaders = await sequencing.findOrderedLeadersBetween(1, 16);
    expect(leaders.length).toBe(4);
  });

  test("sequencing given consensus threshold connected rounds", async () => {
    const NODE_COUNT = 5;
    let pages, page, page1;

    // setup
    const scribes = await Devnet.getPeerids(NODE_COUNT);
    const scribe_keypairs = await Devnet.getKeypairsDict(NODE_COUNT);
    const ds_builder = await NetworkDatastoreBuilder.createInMemory();
    const binder = new DAGRider({
      datastore: ds_builder.datastore,
      election,
    });
    ds_builder.scribes = [...scribes];
    ds_builder.scribe_keypairs = scribe_keypairs;

    // round 1
    await ds_builder.addConsensusConnectedRound();
    page1 = await binder.findLeaderInRound(1);
    expect(page1).toBeNull();

    // round 2
    await ds_builder.addConsensusConnectedRound();
    page1 = await binder.findLeaderInRound(1);
    expect(page1).toBeNull();
    page = await binder.findLeaderInRound(2);
    expect(page).toBeNull();

    // round 3
    await ds_builder.addConsensusConnectedRound();
    page1 = await binder.findLeaderInRound(1);
    expect(page1).toBeNull();
    page = await binder.findLeaderInRound(2);
    expect(page).toBeNull();
    page = await binder.findLeaderInRound(3);
    expect(page).toBeNull();

    // round 4
    await ds_builder.addConsensusConnectedRound();
    page1 = await binder.findLeaderInRound(1);
    expect(page1).not.toBeNull();
    page = await binder.findLeaderInRound(2);
    expect(page).toBeNull();
    page = await binder.findLeaderInRound(3);
    expect(page).toBeNull();
    page = await binder.findLeaderInRound(4);
    expect(page).toBeNull();
    pages = await binder.findOrderedPagesInSection(null, 1);
    expect(pages.length).toBe(1); // first section is only one page
    expect(pages.at(-1).scribe).toBe(page1.scribe);

    // round 8
    await ds_builder.addConsensusConnectedRound();
    await ds_builder.addConsensusConnectedRound();
    await ds_builder.addConsensusConnectedRound();
    await ds_builder.addConsensusConnectedRound();
    pages = await binder.findOrderedPagesInSection(1, 5);
    // given consensus connected rounds, how many nodes in round n-1
    // won't be acked by our nodes in round n?
    const ONE_ROUND_DROPOFF =
      NODE_COUNT - ConsensusMath.calculate2fplus1(NODE_COUNT);
    expect(pages.length).toBe(4 * NODE_COUNT - ONE_ROUND_DROPOFF);
    expect(pages.at(-1).scribe).toBe(scribes[1]);

    // round 12
    await ds_builder.addConsensusConnectedRound();
    await ds_builder.addConsensusConnectedRound();
    await ds_builder.addConsensusConnectedRound();
    await ds_builder.addConsensusConnectedRound();
    pages = await binder.findOrderedPagesInSection(5, 9);
    // further sections still dropoff one page, but also pickup the previously dropped page
    // netting 0 = - ONE_ROUND_DROPOFF + ONE_ROUND_DROPOFF
    // await binder.saveOrderedPageNumbers(1, 9);
    // await ds_builder.datastore.writeToDirectory(process.env.WRITE_TO_DIR);
    expect(pages.length).toBe(4 * NODE_COUNT);
    expect(pages.at(-1).scribe).toBe(scribes[2]);

    // round 16
    await ds_builder.addConsensusConnectedRound();
    await ds_builder.addConsensusConnectedRound();
    await ds_builder.addConsensusConnectedRound();
    await ds_builder.addConsensusConnectedRound();
    pages = await binder.findOrderedPagesInSection(9, 13);
    expect(pages.length).toBe(4 * NODE_COUNT);
    expect(pages.at(-1).scribe).toBe(scribes[3]);
  });

  test("run sequencers", async () => {
    const NODE_COUNT = 9;
    const my_seq_id = Devnet.peeridOf(0);

    const network = await TestNetwork.setup({node_count: NODE_COUNT, sequencing_method: 'DAGRider', election_method: 'RoundRobin'});
    await network.runUntilRound(9);
    const node1 = network.getNode(my_seq_id).runner;

    const leader1 = await node1.sequencing.findLeaderInRound(1);
    expect(leader1).not.toBeNull();
    const leader5 = await node1.sequencing.findLeaderInRound(5);
    expect(leader5).not.toBeNull();
    const pages = await node1.sequencing.findOrderedPagesInSection(null, 5);
    expect(pages.length).toBe(NODE_COUNT * 4 + 1);
  });

  test("given f = 0, one bad sequence, network stalls", async () => {
    const NODE_COUNT = 3;
    const BAD_NODE_COUNT = 1;
    const offline_seq_id = Devnet.peeridOf(NODE_COUNT - 1);

    const network = await TestNetwork.setup({node_count: NODE_COUNT, sequencing_method: 'DAGRider', election_method: 'RoundRobin'});
    network.communication.offline_nodes = [offline_seq_id];

    const abortController = new AbortController();
    setTimeoutPromise(3000).then(() => { abortController.abort() });    
    await expect(network.runUntilRound(9, abortController.signal)).rejects.toThrow("aborted");

    network.communication.offline_nodes = [];
    await network.runUntilRound(9);

    const my_seq_id = Devnet.peeridOf(0);
    const node1 = network.getNode(my_seq_id).runner;
    const leader1 = await node1.sequencing.findLeaderInRound(1);
    expect(leader1).not.toBeNull();
    const leader5 = await node1.sequencing.findLeaderInRound(5);
    expect(leader5).not.toBeNull();
    const pages = await node1.sequencing.findOrderedPagesInSection(null, 5);
    expect(pages.length).toBe((NODE_COUNT) * 4 + 1);
  });

  test("given f = 1, one bad sequencer not elected leader, network can sequence", async () => {
    const NODE_COUNT = 4;
    const BAD_NODE_COUNT = 1;
    const my_seq_id = Devnet.peeridOf(0);
    const offline_seq_id = Devnet.peeridOf(3);

    const network = await TestNetwork.setup({node_count: NODE_COUNT, sequencing_method: 'DAGRider', election_method: 'RoundRobin'});
    network.communication.offline_nodes = [offline_seq_id];
    await network.runUntilRound(9);

    const seq1 = network.getNode(my_seq_id).runner;
    const leader1 = await seq1.sequencing.findLeaderInRound(1);
    expect(leader1).not.toBeNull();
    const leader5 = await seq1.sequencing.findLeaderInRound(5);
    expect(leader5).not.toBeNull();
    const pages = await seq1.sequencing.findOrderedPagesInSection(null, 5);
    expect(pages.length).toBe((NODE_COUNT - BAD_NODE_COUNT) * 4 + 1 + BAD_NODE_COUNT);

    // bring back the offline sequencer
    network.communication.offline_nodes = [];
    await network.runUntilRound(13);
    const pages_r0t9 = await seq1.sequencing.findOrderedPagesInSection(null, 9);
    // bad node not yet producing pages
    expect(pages_r0t9.length).toBe(1 + (NODE_COUNT - BAD_NODE_COUNT) * 8 + 1);

    await network.runUntilRound(17);
    const pages_r0t13 = await seq1.sequencing.findOrderedPagesInSection(null, 13);
    // bad node may have caught up and is producing pages
    expect(pages_r0t13.length).toBeGreaterThanOrEqual(1 + (NODE_COUNT - BAD_NODE_COUNT) * 12 + 1);
  });
});
