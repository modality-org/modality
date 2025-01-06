import JSONStringifyDeterministic from "json-stringify-deterministic";

import Round from "@modality-dev/network-datastore/data/Round";
import ConsensusMath from "../lib/ConsensusMath.js";

export const NAME = "Bullshark";

// like DAGRider, but during periods of synchrony, leaders are chosen twice per wave

/// Bullshark has:
/// * wave round 1 fallback leader (based on randomness of wave round 4, only used during asynchrony)
/// * wave round 1 leader (based on predefined randomness)
/// * wave round 3 leader (based on predefined randomness)
export default class Bullshark {
  constructor({
    datastore,
    election,
    sequencer_first_round = 1,
  }) {
    this.datastore = datastore;
    this.election = election;
    this.sequencer_first_round = sequencer_first_round;
  }

  static create({datastore, election, sequencer_first_round}) {
    return new Bullshark({
      datastore,
      election,
      sequencer_first_round
    });
  }

  async consensusThresholdForRound(round) {
    const scribes = await this.getScribesAtRound(round);
    return ConsensusMath.calculate2fplus1(scribes.length);
  }

  async getScribesAtRound(round) {
    if (round < 1) {
      return [];
      // TODO
      // } else if (round === 1) {
    } else {
      // TODO make this not static
      const round_data = await Round.findOne({
        datastore: this.datastore,
        round: 1,
      });
      return round_data.scribes;
    }
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

  async findFallbackLeaderInRound(round) {
    const round_props = this.constructor.getRoundProps(
      round,
      this.sequencer_first_round
    );

    // only the first round of a wave has a fallback leader
    if (round_props.binder_wave_round !== 1) {
      return null;
    }

    // ensure that rounds r+1,2,3 already complete
    const max_round = await Round.findMaxId({ datastore: this.datastore });
    if (max_round < round + 3) {
      return null;
    }

    // use common coin to pick the leader
    const scribes = await this.getScribesAtRound(round);
    const scribe = await this.election.pickOne({
      options: scribes.sort(),
      input: JSONStringifyDeterministic({
        round: round_props.binder_wave,
        // TODO source of shared randomness
      }),
    });

    const leader = await this.datastore.findPage({ round, scribe });
    if (!leader) {
      return null;
    }

    // ensure that in round+3, 2/3*(scribes) of the pages ack link back to the leader
    let prev_pages = new Set([leader.scribe]);
    let next_pages = new Set();
    for (const i of [1, 2, 3]) {
      for (const i_scribe of scribes) {
        const page = await this.datastore.findPage({
          round: round + i,
          scribe: i_scribe,
        });
        if (page) {
          for (const prev_page of prev_pages) {
            if (page.acks[prev_page]) {
              next_pages.add(page.scribe);
              continue;
            }
          }
        }
      }
      prev_pages = new Set([...next_pages]);
      next_pages = new Set();
    }
    if (prev_pages.size < Math.ceil((2 / 3) * scribes.length)) {
      return null;
    }

    return leader;
  }

  async findFirstSyncLeaderInRound(round) {
    const round_props = this.constructor.getRoundProps(
      round,
      this.sequencer_first_round
    );

    // only the first round of a wave has a first sync leader
    if (round_props.binder_wave_round !== 1) {
      return null;
    }

    // ensure that rounds r+1,2 already complete
    const max_round = await Round.findMaxId({ datastore: this.datastore });
    if (max_round < round + 1) {
      return null;
    }
  }

  async findSecondSyncLeaderInRound(round) {
    const round_props = this.constructor.getRoundProps(
      round,
      this.sequencer_first_round
    );

    // only the third round of a wave has a second sync leader
    if (round_props.binder_wave_round !== 3) {
      return null;
    }

    // ensure that rounds r+1,2 already complete
    const max_round = await Round.findMaxId({ datastore: this.datastore });
    if (max_round < round + 1) {
      return null;
    }
  }

  async findSteadyLeaderInRound(round) {
    const round_props = this.constructor.getRoundProps(
      round,
      this.sequencer_first_round
    );
    if (round_props.binder_wave_round === 1) {
      return this.findFirstSyncLeaderInRound(round);
    } else if (round_props.binder_wave_round === 3) {
      return this.findSecondSyncLeaderInRound(round);
    }
    return null;
  }

  async findLeaderInRound(round) {
    // TODO
    return this.findFallbackLeaderInRound(round);
  }

  async findOrderedLeadersBetween(start_round, end_round) {
    const r = [];
    const start_round_props = this.constructor.getRoundProps(
      start_round,
      this.sequencer_first_round
    );
    let working_round =
      start_round + ((start_round_props.binder_wave_round - 1) % 2);
    while (working_round < end_round) {
      const fallback = await this.findFallbackLeaderInRound(working_round);
      const steady = await this.findSteadyLeaderInRound(working_round);
      r.push({
        round: working_round,
        fallback_scribe: fallback?.scribe,
        steady_scribe: steady?.scribe,
      });
      working_round = working_round + 2;
    }
    return r;
  }

  async findOrderedPagesInSection(start_round, end_round) {
    const starting_leader = await this.findFallbackLeaderInRound(start_round);
    const ending_leader = await this.findFallbackLeaderInRound(end_round);
    return this.datastore.findCausallyLinkedPages(ending_leader, starting_leader);
  }
}
