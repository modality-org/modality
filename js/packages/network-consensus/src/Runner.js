import Block from "@modality-dev/network-datastore/data/Block";
import Round from "@modality-dev/network-datastore/data/Round";
import RoundMessage from "@modality-dev/network-datastore/data/RoundMessage";
import ContractCommitEvent from "@modality-dev/network-datastore/data/ContractCommitEvent";

import ConsensusMath from "./lib/ConsensusMath.js";

import { setTimeout, setImmediate } from "timers/promises";
import { Mutex } from "async-mutex";

const INTRA_ROUND_WAIT_TIME_MS = 50;
const NO_EVENTS_ROUND_WAIT_TIME_MS = 15000;
const NO_EVENTS_POLL_WAIT_TIME_MS = 500;

export default class Runner {
  constructor({
    datastore,
    peerid,
    keypair,
    communication,
    sequencing
  }) {
    this.datastore = datastore;
    this.peerid = peerid;
    this.keypair = keypair;
    this.communication = communication;
    this.sequencing = sequencing;
    this.mutex = new Mutex();
  }

  static create(props) {
    return new Runner(props);
  }

  disableWaiting() {
    this.intra_round_wait_time_ms = 0;
    this.no_events_round_wait_time_ms = 0;
    this.no_events_poll_wait_time_ms = 0;
  }

  async getScribesAtRound(round) {
    return this.sequencing.getScribesAtRound(round);
  }

  async consensusThresholdForRound(round) {
    return this.sequencing.consensusThresholdForRound(round);
  }

  async onReceiveDraftPage(data) {
    const block = await Block.fromJSONObject(data);
    if (!block.validateSig()) {
      console.warn("invalid sig");
      return;
    }

    const round_scribes = await this.getScribesAtRound(block.round_id);
    if (!round_scribes.includes(block.peer_id)) {
      console.warn(
        `ignoring non-scribe ${block.peer_id} at round ${block.round_id}`
      );
      return;
    }

    const current_round = await this.datastore.getCurrentRound();

    if (block.round > current_round) {
      return this.onReceiveDraftPageFromLaterRound(data);
    } else if (block.round < current_round) {
      return this.onReceiveDraftPageFromEarlierRound(data);
    } else {
      return this.onReceiveDraftPageFromCurrentRound(data);
    }
  }

  async onReceiveDraftPageFromEarlierRound(data) {
    const current_round = await this.datastore.getCurrentRound();
    const block = await Block.fromJSONObject(data);
    // console.warn(`received draft for earlier round: round ${page.round} draft received but currently on round ${current_round}`);

    // TODO provide same late ack if asked again

    // provide late ack
    if (this.peerid) {
      const ack = await block.generateLateAck(this.keypair, current_round);
      if (this.communication) {
        const prev_round_certs = await this.datastore.getTimelyCertSigsAtRound(
          current_round - 1
        );
        await this.communication.sendPageLateAck({
          from: this.peerid,
          to: ack.scribe,
          ack_data: ack,
          extra: { prev_round_certs },
        });
      }
      return ack;
    }
  }

  async onReceiveDraftPageFromLaterRound(data) {
    const current_round = await this.datastore.getCurrentRound();
    const page = await Block.fromJSONObject(data);
    // console.warn(`received draft for later round: round ${page.round} draft received but currently on round ${current_round}`);

    await RoundMessage.fromJSONObject({
      round: page.round,
      scribe: page.scribe,
      type: "draft",
      seen_at_round: current_round,
      content: data,
    }).save({ datastore: this.datastore });

    // TODO considering bumping rounds!
    // TODO req and verify acker's prev_round_certs chain
    if (current_round < page.round) {
      if (
        !this.latest_seen_at_round ||
        data.round_id > this.latest_seen_at_round
      ) {
        this.latest_seen_at_round = data.round_id;
        return;
      }
    }
  }

  async onReceiveDraftPageFromCurrentRound(data) {
    const block = await Block.fromJSONObject(data);

    if (this.peerid) {
      const ack = await block.generateAck(this.keypair);
      if (this.communication) {
        await this.communication.sendPageAck({
          from: this.peerid,
          to: ack.acker,
          ack_data: ack,
        });
      }
      return ack;
    }
  }

  async onReceivePageAck(ack) {
    if (!ack) {
      return;
    }

    const whoami = this.peerid;
    if (!whoami || whoami !== ack.peer_id) {
      return;
    }

    const round_id = await this.datastore.getCurrentRound();
    if (ack.round_id !== round_id) {
      return;
    }

    const round_scribes = await this.getScribesAtRound(round_id);
    if (!round_scribes.includes(ack.acker)) {
      console.warn(
        `ignoring non-scribe ack ${ack.acker} at round ${ack.round}`
      );
      return;
    }

    const block = await Block.findOne({
      datastore: this.datastore,
      round_id,
      peer_id: whoami,
    });
    if (block) {
      await this.mutex.runExclusive(async () => {
        await block.addAck(ack);
        await block.save({ datastore: this.datastore });
      });
    }
  }

  async onReceivePageLateAck(ack) {
    return;
  }

  async onReceiveCertifiedPage(data) {
    const block = await Block.fromJSONObject(data);
    if (!block.validateSig()) {
      return null;
    }

    const round = await this.datastore.getCurrentRound();
    if (block.round_id < round) {
      // console.log({round}, data);
      // return this.onReceiveLateCertifiedPage(data);
    } else if (block.round_id > round) {
      return this.onReceiveCertifiedPageFromLaterRound(data);
    }

    return this.onReceiveCertifiedPageFromCurrentRound(data);
  }

  async onReceiveCertifiedPageFromLaterRound(data) {
    const current_round = await this.datastore.getCurrentRound();
    const block = await Block.fromJSONObject(data);

    await RoundMessage.fromJSONObject({
      round: block.round_id,
      scribe: block.peer_id,
      type: "certified",
      seen_at_round: current_round,
      content: data,
    }).save({ datastore: this.datastore });

    // TODO considering bumping rounds!
    // TODO req and verify acker's prev_round_certs chain
    if (current_round < block.round_id) {
      if (
        !this.latest_seen_at_round ||
        data.round_id > this.latest_seen_at_round
      ) {
        this.latest_seen_at_round = data.round_id;
        return;
      }
    }
  }

  async onReceiveCertifiedPageFromCurrentRound(data) {
    const block = await Block.fromJSONObject(data);
    if (!block.validateSig()) {
      return null;
    }
    const round_id = block.round_id;

    const last_round_threshold = await this.consensusThresholdForRound(
      round_id - 1
    );
    const current_round_threshold =
      await this.consensusThresholdForRound(round_id);

    if (
      round_id > 1 &&
      Object.keys(block.prev_round_certs).length < last_round_threshold
    ) {
      return null;
    }

    const has_valid_cert = await block.validateCert({
      acks_needed: current_round_threshold,
    });
    if (!has_valid_cert) {
      return null;
    }

    await block.save({ datastore: this.datastore });
    return block;
  }

  async getOrFetchPrevRoundCerts(round) {
    const prev_round = round - 1;
    let prev_round_certs = await this.datastore.getTimelyCertSigsAtRound(prev_round);
    const prev_round_scribes = await this.getScribesAtRound(prev_round);
    const threshold = ConsensusMath.calculate2fplus1(
      prev_round_scribes.length
    );
    if (Object.keys(prev_round_certs) >= threshold) {
      return prev_round_certs;
    }

    if (this.communication) {
      for (const scribe of prev_round_scribes) {
        const block_data =
          await this.communication.fetchScribeRoundCertifiedPage({
            from: this.peerid,
            to: scribe,
            scribe,
            round: prev_round,
          });
        if (block_data) {
          const page = await Block.fromJSONObject(block_data);
          if (page.validateCert({ acks_needed: threshold })) {
            await page.save({ datastore: this.datastore });
          }
        }
      }
    }

    prev_round_certs = await this.datastore.getTimelyCertSigsAtRound(prev_round);

    return prev_round_certs;
  }

  async speedUpToLatestUncertifiedRound() {
    let round_certified = true;
    let round = await this.datastore.getCurrentRound() + 1;
    while (round_certified) {
      const prev_round_certs = await this.getOrFetchPrevRoundCerts(round);
      const existing_certs = await RoundMessage.findAllInRoundOfType({
        datastore: this.datastore,
        round: round - 1,
        type: "certified",
      });
      for (const draft of existing_certs) {
        const draft_content = draft.content;
        await this.datastore.datastore.delete(draft.getId());
        await this.onReceiveCertifiedPage(draft_content);
      }
      const threshold = await this.consensusThresholdForRound(round - 1);
      const cert_count = Object.keys(prev_round_certs).length;
      if (cert_count && threshold && cert_count >= threshold) {
        round = round + 1;
      } else {
        round_certified = false;
      }
    }
    const newest_uncertified_round = round - 1;
    await this.datastore.setCurrentRound(newest_uncertified_round);
  }

  async runRound(signal) {
    await this.speedUpToLatestUncertifiedRound();
    let round = await this.datastore.getCurrentRound();

    const prev_round_certs = await this.getOrFetchPrevRoundCerts(round);

    const threshold = await this.consensusThresholdForRound(round - 1);
    const cert_count = Object.keys(prev_round_certs).length;
    if (cert_count < threshold) {
      throw new Error("not enough certs to start round");
    }

    const current_round_threshold = await this.consensusThresholdForRound(round);
    const existing_this_round_certs = await RoundMessage.findAllInRoundOfType({
      datastore: this.datastore,
      round: round,
      type: "certified",
    });
    if (existing_this_round_certs.length >= current_round_threshold) {
      await this.bumpCurrentRound();
      round = await this.datastore.getCurrentRound();
    }

    let cc_events = await ContractCommitEvent.findAll({ datastore: this.datastore });
    let keep_waiting_for_events = (cc_events.length === 0);
    if (keep_waiting_for_events) {
      setTimeout(this.no_events_round_wait_time_ms ?? NO_EVENTS_ROUND_WAIT_TIME_MS).then(() => {
        keep_waiting_for_events = false;
      });
    }
    while (keep_waiting_for_events) {
      await setTimeout(this.no_events_poll_wait_time_ms ?? NO_EVENTS_POLL_WAIT_TIME_MS);
      cc_events = await ContractCommitEvent.findAll({ datastore: this.datastore });
      if (cc_events.length > 0) {
        keep_waiting_for_events = false;
      }
    }
    const events = [];
    for (const cc_event of cc_events) {
      events.push({
        contract_id: cc_event.contract_id,
        commit_id: cc_event.commit_id,
      });
      await cc_event.delete({ datastore: this.datastore });
    }
    const block = Block.from({
      round_id: round,
      peer_id: this.peerid,
      prev_round_certs,
      events,
    });
    await block.generateSig(this.keypair);
    await block.save({ datastore: this.datastore });

    if (this.communication) {
      const block_data = await block.toDraftJSONObject();
      await this.communication.broadcastDraftPage({
        from: this.peerid,
        page_data: block_data,
      });
    }

    // handle enqueue round messages
    const existing_drafts = await RoundMessage.findAllInRoundOfType({
      datastore: this.datastore,
      round,
      type: "draft",
    });
    for (const draft of existing_drafts) {
      const draft_content = draft.content;
      await this.datastore.datastore.delete(draft.getId());
      await this.onReceiveDraftPage(draft_content);
    }

    let keep_waiting_for_acks = this.latest_seen_at_round ? false : true;
    let keep_waiting_for_certs = true;
    while (keep_waiting_for_acks || keep_waiting_for_certs) {
      if (this.latest_seen_at_round && this.latest_seen_at_round > round) {
        await this.jumpToRound(
          this.latest_seen_at_round,
          this.latest_seen_at_last_round_certs
        );
        this.latest_seen_at_round = null;
        return;
      }
      if (signal?.aborted) {
        throw new Error("aborted");
      }
      if (keep_waiting_for_acks) {
        await block.reload({ datastore: this.datastore });
        const valid_acks = await block.countValidAcks();
        if (valid_acks >= current_round_threshold) {
          await block.generateCert(this.keypair);
          if (this.communication) {
            await this.communication.broadcastCertifiedPage({
              from: this.peerid,
              page_data: await block.toJSONObject(),
            });
          }
          keep_waiting_for_acks = false;
        }
      }
      if (keep_waiting_for_certs) {
        const current_round_certs =
          await this.datastore.getTimelyCertsAtRound(round);
        if (
          Object.keys(current_round_certs).length >= current_round_threshold
        ) {
          keep_waiting_for_certs = false;
        }
      }
      const wait_in_ms =
        this.intra_round_wait_time_ms ?? INTRA_ROUND_WAIT_TIME_MS;
      if (wait_in_ms) {
        await setTimeout(wait_in_ms);
      } else {
        await setImmediate();
      }
    }
    await this.bumpCurrentRound();
  }

  async onFetchScribeRoundCertifiedPageRequest({ round, scribe }) {
    return this.datastore.findPage({ round, scribe });
  }

  async requestRoundDataFromPeers(round) {
    const scribes = await this.getScribesAtRound(round);
    for (const scribe of scribes) {
      const page = await this.communication.fetchScribeRoundCertifiedPage({
        from: this.peerid,
        to: scribe,
        scribe,
        round,
      });
      if (page) {
        await this.onReceiveCertifiedPage(page);
      }
    }
  }

  async jumpToRound(round_num) {
    const current_round_num = await this.datastore.getCurrentRound();
    for (let i = current_round_num + 1; i < round_num; i++) {
      // TODO maybe handle jumping from earlier rounds
      // const roundi = Round.from({ round_id: i });
      // roundi.scribes = await this.getScribesAtRound(i);
      // await roundi.save({ datastore: this.datastore });
    }
    const round = Round.from({ round_id: round_num });
    round.scribes = await this.getScribesAtRound(round_num);
    await round.save({ datastore: this.datastore });
    await this.datastore.setCurrentRound(round_num);
  }

  async bumpCurrentRound() {
    const round_num = await this.datastore.getCurrentRound();
    const round = Round.from({ round_id: round_num });
    round.scribes = await this.getScribesAtRound(round_num);
    await round.save({ datastore: this.datastore });
    await this.datastore.bumpCurrentRound();
  }

  async runUntilRound(round, signal) {
    let current_round = await this.datastore.getCurrentRound();
    while (current_round < round) {
      if (signal?.aborted) {
        throw new Error("aborted");
      }
      await this.runRound(signal);
      current_round = await this.datastore.getCurrentRound();
    }
  }

  async run(signal) {
    // eslint-disable-next-line no-constant-condition
    while (true) {
      await this.runRound(signal);
    }
  }
}
