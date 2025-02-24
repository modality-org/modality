import JSONStringifyDeterministic from "json-stringify-deterministic";

import Block from "@modality-dev/network-datastore/data/Block";
import ConsensusMath from "../lib/ConsensusMath.js";

export const NAME = "DAGRider";

export default class DAGRider {
  constructor({ datastore, election, sequencer_first_round = 1 }) {
    this.datastore = datastore;
    this.election = election;
    this.sequencer_first_round = sequencer_first_round;
  }

  static create({ datastore, election, sequencer_first_round }) {
    return new DAGRider({
      datastore,
      election,
      sequencer_first_round,
    });
  }

  async consensusThresholdForRound(round) {
    const scribes = await this.getScribesAtRound(round);
    return ConsensusMath.calculate2fplus1(scribes.length);
  }

  async getScribesAtRound(round) {
    return []; // TODO
    // if (round < 1) {
    //   return [];
    //   // TODO
    //   // } else if (round === 1) {
    // } else {
    //   // TODO make this not static
    //   const round_data = await Round.findOne({
    //     datastore: this.datastore,
    //     round_id: 1,
    //   });
    //   return round_data.scribes;
    // }
  }

  static getBoundRound(round, sequencer_first_round = 1) {
    return round - sequencer_first_round;
  }

  static getWaveOfRound(round, sequencer_first_round = 1) {
    const bound_round = this.getBoundRound(round, sequencer_first_round);
    return Math.floor(bound_round / 4) + 1;
  }

  static getWaveRoundOfRound(round, sequencer_first_round) {
    const bound_round = this.getBoundRound(round, sequencer_first_round);
    return (bound_round % 4) + 1;
  }

  static getRoundProps(round, sequencer_first_round) {
    const binder_round = round - sequencer_first_round + 1;
    const binder_wave = this.getWaveOfRound(round, sequencer_first_round);
    const binder_wave_round = this.getWaveRoundOfRound(
      round,
      sequencer_first_round
    );
    return {
      round,
      binder_round,
      binder_wave,
      binder_wave_round,
    };
  }

  async findLeaderInRound(round) {
    const round_props = this.constructor.getRoundProps(
      round,
      this.sequencer_first_round
    );

    // only the first round of a wave has an leader
    if (round_props.binder_wave_round !== 1) {
      return null;
    }

    // ensure that rounds r+1,2,3 already complete
    const max_round = await this.datastore.getCurrentRound();
    if (max_round < round + 3) {
      return null;
    }

    // use common coin to pick the leader
    const scribes = await this.getScribesAtRound(round);
    const scribe = await this.election.pickOne({
      options: scribes.sort(),
      input: JSONStringifyDeterministic({
        round: round_props.binder_wave,
        // TODO source of shared election
      }),
    });

    const leader = await this.datastore.findBlock({ round, scribe });
    // console.log({ round, scribes, scribe, leader });
    if (!leader) {
      return null;
    }

    // ensure that in round+3, 2/3*(scribes) of the blocks link back to the leader thru certs
    let prev_round_scribes = new Set([leader.scribe]);
    let next_round_scribes = new Set();
    for (const i of [1, 2, 3]) {
      // TODO support changes in scribes
      for (const i_scribe of scribes) {
        const block = await this.datastore.findBlock({
          round: round + i,
          scribe: i_scribe,
        });
        if (block) {
          for (const prev_block_scribe of prev_round_scribes) {
            const prev_block = await this.datastore.findBlock({
              round: round + i - 1,
              scribe: prev_block_scribe,
            });
            if (
              block.last_round_certs[prev_block.scribe]?.cert ===
              prev_block.cert
            ) {
              next_round_scribes.add(block.scribe);
              continue;
            }
          }
        }
      }
      prev_round_scribes = new Set([...next_round_scribes]);
      next_round_scribes = new Set();
    }
    if (prev_round_scribes.size < Math.ceil((2 / 3) * scribes.length)) {
      return null;
    }

    return leader;
  }

  async findOrderedLeadersBetween(start_round, end_round) {
    const r = [];
    const start_round_props = this.constructor.getRoundProps(
      start_round,
      this.sequencer_first_round
    );
    let working_round =
      start_round +
      (start_round_props.binder_wave_round === 1
        ? 0
        : 5 - start_round_props.binder_wave_round);
    while (working_round < end_round) {
      const block = await this.findLeaderInRound(working_round);
      r.push({ round: working_round, scribe: block.scribe });
      working_round = working_round + 4;
    }
    return r;
  }

  async findOrderedBlocksInSection(start_round, end_round) {
    const starting_leader = await this.findLeaderInRound(start_round);
    const ending_leader = await this.findLeaderInRound(end_round);
    // console.log({start_round, starting_leader, end_round, ending_leader});
    return this.datastore.findCausallyLinkedBlocks(
      ending_leader,
      starting_leader
    );
  }

  async findOrderedBlocksUptoRound(end_round) {
    const start_round = 1;
    const round_section_leaders = [];
    for (let round = start_round; round <= end_round; round++) {
      const leader = await this.findLeaderInRound(round);
      if (leader) {
        round_section_leaders.push(leader);
      }
    }
    if (!round_section_leaders.length) {
      return;
    }
    let prev_leader;
    let block_number;
    if (start_round === 1) {
      block_number = 1;
    }
    let r = [];
    for (const leader of round_section_leaders) {
      if (!prev_leader) {
        prev_leader = leader;
        continue;
      }
      const ordered_blocks = await this.findOrderedBlocksInSection(
        prev_leader.round,
        leader.round
      );
      r = [...r, ...ordered_blocks];
    }
    return r;
  }

  async saveOrderedBlockNumbers(start_round, end_round) {
    const round_section_leaders = [];
    for (let round = start_round; round < end_round; round++) {
      const leader = await this.findLeaderInRound(round);
      if (leader) {
        round_section_leaders.push(leader);
      }
    }
    if (!round_section_leaders.length) {
      return;
    }
    const ordered_section_blocks = [];
    let prev_leader;
    let block_number;
    if (start_round === 1) {
      block_number = 1;
    }
    for (const leader of round_section_leaders) {
      if (!prev_leader) {
        prev_leader = leader;
        continue;
      }
      const ordered_blocks = await this.findOrderedBlocksInSection(
        prev_leader.round,
        leader.round
      );
      const section_starting_round = prev_leader.round;
      const section_ending_round = leader.round;
      ordered_section_blocks.push({
        section_starting_round: prev_leader.round,
        section_ending_round: leader.round,
        blocks: ordered_blocks,
      });
      let section_block_number = 1;
      for (const ordered_block of ordered_blocks) {
        const block = await Block.findOne({
          datastore: this.datastore,
          round: ordered_block.round,
          scribe: ordered_block.scribe,
        });
        block.section_starting_round = section_starting_round;
        block.section_ending_round = section_ending_round;
        block.section_block_number = section_block_number;
        if (block_number) {
          block.block_number = block_number;
        }
        await block.save({ datastore: this.datastore });
        section_block_number++;
        if (block_number) {
          block_number++;
        }
      }
      prev_leader = leader;
    }
  }
}
