import NetworkDatastore from "./NetworkDatastore.js";

import Block from "./data/Block.js";
import Round from "./data/Round.js";
import fs from "fs";
import Keypair from "@modality-dev/utils/Keypair";
import DevnetCommon from "../../network-configs/src/devnet-common/index.js";

function shuffleArray(array) {
  for (let i = array.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [array[i], array[j]] = [array[j], array[i]];
  }
  return array;
}

function randomSetOfN(input, n) {
  let all_set;
  if (input instanceof Set) {
    all_set = input;
  } else {
    all_set = new Set(input);
  }
  return shuffleArray(Array.from(all_set)).slice(0, n);
}

function randomSetOfNIncluding(array, n, include_array) {
  const all_set = new Set(array);
  const include_set = new Set(include_array);
  const all_minus_include_set = all_set.difference(include_set);
  const random_set = randomSetOfN(
    all_minus_include_set,
    n - include_array.length
  );
  return new Set([...include_array, ...random_set]);
}

export default class NetworkDatastoreBuilder {
  constructor() {
    this.datastore = null;
    this.round_num = 0;
    this.scribe_keypairs = {};
    this.scribes = [];
    this.late_acks = {};
    this.next_round_late_acks = {};
  }

  static async generateScribes(count, from_devnet_common = false) {
    const r = {};
    if (from_devnet_common) {
      const keypairs = Object.values(DevnetCommon.keypairs).slice(0, count);
      for (const keypair of keypairs) {
        r[keypair.id] = await Keypair.fromJSON(keypair);
      }
    } else {
      for (let i = 1; i <= count; i++) {
        const keypair = await Keypair.generate();
        const keypair_peerid = await keypair.asPublicAddress();
        r[keypair_peerid] = keypair;
      }
    }
    return r;
  }

  async generateScribes(count, from_devnet_common = false) {
    this.scribe_keypairs = await this.constructor.generateScribes(
      count,
      from_devnet_common
    );
    this.scribes = Object.keys(this.scribe_keypairs);
  }

  static async createInMemory() {
    const builder = new NetworkDatastoreBuilder();
    builder.datastore = await NetworkDatastore.createInMemory();
    return builder;
  }

  static async createInDirectory(path) {
    if (!fs.existsSync(path)) {
      fs.mkdirSync(path, { recursive: true });
    }
    const builder = new NetworkDatastoreBuilder();
    builder.datastore = await NetworkDatastore.createInDirectory(path);
    return builder;
  }

  async createSequencers(SeqType, opts = {}) {
    const r = {};
    for (const scribe of this.scribes) {
      const seq = new SeqType({
        datastore: await this.datastore.cloneToMemory(),
        peerid: scribe,
        keypair: this.scribe_keypairs[scribe],
        ...opts,
      });
      r[scribe] = seq;
    }
    return r;
  }

  async setupGenesisScribes(scribe_keypairs, initial_rounds = 1) {
    this.scribe_keypairs = scribe_keypairs;
    this.scribes = Object.keys(scribe_keypairs);
    for (let i = 0; i < initial_rounds; i++) {
      await this.addFullyConnectedRound();
      this.datastore.setCurrentRound(i + 1);
    }
  }

  async addFullyConnectedRound({ failures = 0 } = {}) {
    const round_num = ++this.round_num;
    const round = Round.from({ round_id: round_num });
    round.scribes = [...this.scribes];
    await round.save({ datastore: this.datastore });
    const scribes = shuffleArray(this.scribes);
    for (const scribe of scribes) {
      if (failures > 0) {
        failures--;
        continue;
      }
      let last_round_certs = {};
      if (round_num > 1) {
        for (const peer_scribe of scribes) {
          const peer_prev_block = await Block.findOne({
            datastore: this.datastore,
            round_id: round_num - 1,
            peer_id: peer_scribe,
          });
          last_round_certs[peer_prev_block.scribe] = {
            peer_id: peer_prev_block.scribe,
            cert: peer_prev_block.cert,
          };
        }
      }
      const block = Block.from({
        peer_id: scribe,
        round_id: round_num,
        prev_round_certs: last_round_certs,
        events: [],
      });
      await block.generateSig(this.scribe_keypairs[scribe]);
      for (const peer_scribe of scribes) {
        await block.addAck(
          await block.generateAck(this.scribe_keypairs[peer_scribe])
        );
      }
      await block.generateCert(this.scribe_keypairs[scribe]);
      await block.save({ datastore: this.datastore });
    }
    await this.datastore.bumpCurrentRound();
  }

  async addConsensusConnectedRound({ failures = 0 } = {}) {
    const round_num = ++this.round_num;
    const round = Round.from({ round: round_num });
    round.scribes = [...this.scribes];
    await round.save({ datastore: this.datastore });
    const scribes = shuffleArray(this.scribes);
    const consensus_threshold =
      Math.floor((this.scribes.length * 2.0) / 3.0) + 1;
    for (const scribe of scribes) {
      if (failures > 0) {
        failures--;
        continue;
      }
      let last_round_certs = {};
      if (round_num > 1) {
        const last_round_certified_scribes = randomSetOfNIncluding(
          scribes,
          consensus_threshold,
          [scribe]
        );
        for (const peer_scribe of last_round_certified_scribes) {
          const peer_prev_block = await Block.findOne({
            datastore: this.datastore,
            round_id: round_num - 1,
            peer_id: peer_scribe,
          });
          last_round_certs[peer_prev_block.scribe] = {
            peer_id: peer_prev_block.scribe,
            cert: peer_prev_block.cert,
          };
        }
      }
      const block = Block.from({
        peer_id: scribe,
        round_id: round_num,
        prev_round_certs: last_round_certs,
        events: [],
      });
      const acking_scribes = randomSetOfNIncluding(
        scribes,
        consensus_threshold,
        [scribe]
      );
      for (const peer_scribe of acking_scribes) {
        await block.addAck(
          await block.generateAck(this.scribe_keypairs[peer_scribe])
        );
      }
      await block.generateCert(this.scribe_keypairs[scribe]);
      await block.save({ datastore: this.datastore });

      await block.save({ datastore: this.datastore });
    }
    this.late_acks = this.next_round_late_acks;
    this.next_round_late_acks = {};
    await this.datastore.bumpCurrentRound();
  }

  async addPartiallyConnectedRound({ failures = 0 } = {}) {
    const round_num = ++this.round_num;
    const round = Round.from({ round_id: round_num });
    round.scribes = [...this.scribes];
    await round.save({ datastore: this.datastore });
    const scribes = shuffleArray(this.scribes);
    const consensus_threshold =
      Math.floor((this.scribes.length * 2.0) / 3.0) + 1;
    for (const scribe of scribes) {
      if (failures > 0) {
        failures--;
        continue;
      }
      const block = Block.from({ scribe, round: round_num, events: [] });
      if (round_num > 1) {
        // prioritize self ack
        const acking_scribes = [
          scribe,
          ...shuffleArray([...scribes].filter((i) => i !== scribe)),
        ];
        let acks_so_far = 0;
        for (const peer_scribe of acking_scribes) {
          const peer_prev_block = await Block.findOne({
            datastore: this.datastore,
            round: round_num - 1,
            scribe: peer_scribe,
          });
          if (peer_prev_block) {
            if (acks_so_far >= consensus_threshold) {
              this.next_round_late_acks[scribe] = [
                ...(this.next_round_late_acks[scribe] || []),
                {
                  round: peer_prev_block?.round,
                  scribe: peer_scribe,
                },
              ];
            } else {
              await block.addAck(
                await block.generateAck(this.scribe_keypairs[peer_scribe])
              );
              acks_so_far++;
            }
          }
        }
      }
      const late_acks = this.late_acks[scribe] || [];
      for (const late_ack of late_acks) {
        await block.addLateAck(late_ack);
      }
      this.late_acks[scribe] = [];
      await block.save({ datastore: this.datastore });
    }
    this.late_acks = this.next_round_late_acks;
    this.next_round_late_acks = {};
  }
}
