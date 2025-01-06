import Devnet from "@modality-dev/network-configs/Devnet";
import NetworkDatastoreBuilder from "@modality-dev/network-datastore/NetworkDatastoreBuilder";
import SameProcess from "../src/communication/SameProcess";

import Runner from "../src/Runner";
import { SEQUENCING_METHODS } from "../src/sequencing";
import { ELECTION_METHODS } from "../src/election";

export default class TestNetwork {
  constructor() {
    this.nodes = {};
  }

  static async setup({node_count = 1, election_method, sequencing_method}) {
    const tn = new TestNetwork();
    tn.communication = new SameProcess();
    tn.communication.nodes = {};

    const scribes = await Devnet.getPeerids(node_count);
    const scribe_keypairs = await Devnet.getKeypairsDict(node_count);
    const ds_builder = await NetworkDatastoreBuilder.createInMemory();
    ds_builder.scribes = [...scribes];
    ds_builder.scribe_keypairs = scribe_keypairs;
    await ds_builder.addFullyConnectedRound();

    const election = ELECTION_METHODS[election_method].create();
    for (const scribe of scribes) {
      tn.nodes[scribe] = {};
      tn.nodes[scribe].datastore = await ds_builder.datastore.cloneToMemory();

      const sequencing = SEQUENCING_METHODS[sequencing_method].create({
        datastore: tn.nodes[scribe].datastore,
        peerid: scribe,
        keypair: scribe_keypairs[scribe],
        election,
      });

      const runner = Runner.create({
        communication: tn.communication,
        datastore: tn.nodes[scribe].datastore,
        peerid: scribe,
        keypair: scribe_keypairs[scribe],
        communication_enabled: true,
        sequencing,
      });

      runner.intra_round_wait_time_ms = 0;
      runner.no_events_round_wait_time_ms = 0;
      runner.no_events_poll_wait_time_ms = 0;

      tn.communication.nodes[scribe] = runner;
      tn.nodes[scribe].runner = runner;
    }

    return tn;
  }

  getNode(pubkey) {
    return this.nodes[pubkey];
  }

  onlineSequencerEntries() {
    return Object.fromEntries(Object.entries(this.nodes).filter((seq) => !this.communication.offline_nodes.includes(seq[0])));
  }

  runUntilRound(round, signal) {
    return Promise.all(Object.values(this.onlineSequencerEntries()).map((seq) => seq.runner.runUntilRound(round, signal)));
  }
}