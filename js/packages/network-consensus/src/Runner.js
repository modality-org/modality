import Block from "@modality-dev/network-datastore/data/Block";
import BlockMessage from "@modality-dev/network-datastore/data/BlockMessage";
import Transaction from "@modality-dev/network-datastore/data/Transaction";

import ConsensusMath from "./lib/ConsensusMath.js";

import { setTimeout, setImmediate } from "timers/promises";
import { Mutex } from "async-mutex";

const INTRA_ROUND_WAIT_TIME_MS = 50;
const NO_EVENTS_ROUND_WAIT_TIME_MS = 15000;
const NO_EVENTS_POLL_WAIT_TIME_MS = 500;
const KEEP_WAITING_LOG_INTERVAL_MS = 5000;
const KEEP_WAITING_FOR_ACKS_REBROADCAST_INTERVAL_MS = 5000;

export default class Runner {
  constructor({ datastore, peerid, keypair, communication, sequencing }) {
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

  async onReceiveBlockDraft(data) {
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
      return this.onReceiveDraftBlockFromLaterRound(data);
    } else if (block.round < current_round) {
      return this.onReceiveDraftBlockFromEarlierRound(data);
    } else {
      return this.onReceiveDraftBlockFromCurrentRound(data);
    }
  }

  async onReceiveDraftBlockFromEarlierRound(data) {
    const current_round = await this.datastore.getCurrentRound();
    const block = await Block.fromJSONObject(data);
    // console.warn(`received draft for earlier round: round ${block.round} draft received but currently on round ${current_round}`);

    // TODO provide same late ack if asked again

    // provide late ack
    if (this.peerid) {
      const ack = await block.generateLateAck(this.keypair, current_round);
      if (this.communication) {
        const prev_round_certs = await this.datastore.getTimelyCertSigsAtRound(
          current_round - 1
        );
        await this.communication.sendBlockLateAck({
          from: this.peerid,
          to: ack.scribe,
          ack_data: ack,
          extra: { prev_round_certs },
        });
      }
      return ack;
    }
  }

  async onReceiveDraftBlockFromLaterRound(data) {
    const current_round = await this.datastore.getCurrentRound();
    const block = await Block.fromJSONObject(data);
    // console.warn(`received draft for later round: round ${block.round} draft received but currently on round ${current_round}`);

    await BlockMessage.fromJSONObject({
      round: block.round,
      scribe: block.scribe,
      type: "draft",
      seen_at_round: current_round,
      content: data,
    }).save({ datastore: this.datastore });

    // TODO considering bumping rounds!
    // TODO req and verify acker's prev_round_certs chain
    if (current_round < block.round) {
      if (
        !this.latest_seen_at_round ||
        data.round_id > this.latest_seen_at_round
      ) {
        this.latest_seen_at_round = data.round_id;
        return;
      }
    }
  }

  async onReceiveDraftBlockFromCurrentRound(data) {
    const block = await Block.fromJSONObject(data);

    if (this.peerid) {
      const ack = await block.generateAck(this.keypair);
      if (this.communication) {
        await this.communication.sendBlockAck({
          from: this.peerid,
          to: ack.peer_id,
          ack_data: ack,
        });
      }
      return ack;
    }
  }

  async onReceiveBlockAck(ack) {
    if (!ack) {
      return;
    }

    const whoami = this.peerid;
    if (!whoami) {
      console.warn(`ignoring ack, no private key`);
      return;
    }

    const round_id = await this.datastore.getCurrentRound();
    if (ack.round_id != round_id) {
      console.warn(`ignoring ack from other round`);
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
        await block.reload({ datastore: this.datastore });
        await block.addAck(ack);
        await block.save({ datastore: this.datastore });
      });
    }
  }

  async onReceiveBlockLateAck(ack) {
    return;
  }

  async onReceiveBlockCert(data) {
    const block = await Block.fromJSONObject(data);
    if (!block.validateSig()) {
      return null;
    }

    const round = await this.datastore.getCurrentRound();
    if (block.round_id < round) {
      // console.log({round}, data);
      // return this.onReceiveLateCertifiedBlock(data);
    } else if (block.round_id > round) {
      return this.onReceiveCertifiedBlockFromLaterRound(data);
    }

    return this.onReceiveCertifiedBlockFromCurrentRound(data);
  }

  async onReceiveCertifiedBlockFromLaterRound(data) {
    const current_round = await this.datastore.getCurrentRound();
    const block = await Block.fromJSONObject(data);

    await BlockMessage.fromJSONObject({
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

  async onReceiveCertifiedBlockFromCurrentRound(data) {
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
    if (round == 0) {
      return {};
    }
    const prev_round = round - 1;
    let prev_round_certs =
      await this.datastore.getTimelyCertSigsAtRound(prev_round);
    const prev_round_scribes = await this.getScribesAtRound(prev_round);
    const threshold = ConsensusMath.calculate2fplus1(prev_round_scribes.length);
    if (Object.keys(prev_round_certs) >= threshold) {
      return prev_round_certs;
    }

    if (this.communication) {
      for (const peer_id of prev_round_scribes) {
        let block_data = (
          await this.communication.fetchScribeRoundCertifiedBlock({
            from: this.peerid,
            to: peer_id,
            round_id: round,
            peer_id,
          })
        )?.block;
        if (!block_data) {
          for (const alt_peer_id of prev_round_scribes) {
            block_data = (
              await this.communication.fetchScribeRoundCertifiedBlock({
                from: this.peerid,
                to: alt_peer_id,
                round_id: round,
                peer_id,
              })
            )?.block;
            // console.log({to: alt_peer_id, round, peer_id})
            if (block_data) { break; }
          }
        }
        if (block_data) {
          const block = await Block.fromJSONObject(block_data);
          if (block.validateCert({ acks_needed: threshold })) {
            await block.save({ datastore: this.datastore });
          }
        }
      }
    }

    prev_round_certs =
      await this.datastore.getTimelyCertSigsAtRound(prev_round);

    return prev_round_certs;
  }

  async speedUpToLatestUncertifiedRound() {
    let round_certified = true;
    let round = (await this.datastore.getCurrentRound()) + 1;
    while (round_certified) {
      const prev_round_certs = await this.getOrFetchPrevRoundCerts(round);
      const existing_certs = await BlockMessage.findAllInRoundOfType({
        datastore: this.datastore,
        round: round - 1,
        type: "certified",
      });
      for (const draft of existing_certs) {
        const draft_content = draft.content;
        await this.datastore.datastore.delete(draft.getId());
        await this.onReceiveBlockCert(draft_content);
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

    let hasCertsRequired = false;
    let working_round = round;
    while (!hasCertsRequired) {
      if (working_round < 1) {
        break;
      }
      const prev_round_certs = await this.getOrFetchPrevRoundCerts(working_round);
      const threshold = await this.consensusThresholdForRound(working_round - 1);
      const cert_count = Object.keys(prev_round_certs).length;
      if (cert_count >= threshold) {
        break;
      } else {
        console.log(`NOT ENOUGH ${cert_count}/${threshold} going back to round ${working_round - 1}`)
        working_round = working_round - 1;
      }
    }

    const prev_round_certs = await this.getOrFetchPrevRoundCerts(round);
    const threshold = await this.consensusThresholdForRound(round - 1);
    const cert_count = Object.keys(prev_round_certs).length;
    if (cert_count < threshold) {
      console.warn({
        prev_round: round - 1,
        cert_count,
        threshold,
        prev_round_certs,
      });
      throw new Error("not enough certs to start round");
    }

    const current_round_threshold =
      await this.consensusThresholdForRound(round);
    const existing_this_round_certs = await BlockMessage.findAllInRoundOfType({
      datastore: this.datastore,
      round: round,
      type: "certified",
    });
    if (existing_this_round_certs.length >= current_round_threshold) {
      await this.bumpCurrentRound();
      round = await this.datastore.getCurrentRound();
    }

    let cc_events = await Transaction.findAll({ datastore: this.datastore });
    let keep_waiting_for_events = cc_events.length === 0;
    if (keep_waiting_for_events) {
      setTimeout(
        this.no_events_round_wait_time_ms ?? NO_EVENTS_ROUND_WAIT_TIME_MS
      ).then(() => {
        keep_waiting_for_events = false;
      });
    }
    while (keep_waiting_for_events) {
      await setTimeout(
        this.no_events_poll_wait_time_ms ?? NO_EVENTS_POLL_WAIT_TIME_MS
      );
      cc_events = await Transaction.findAll({ datastore: this.datastore });
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
      await this.communication.broadcastDraftBlock({
        from: this.peerid,
        block_data: block_data,
      });
    }

    // handle enqueue round messages
    const existing_drafts = await BlockMessage.findAllInRoundOfType({
      datastore: this.datastore,
      round,
      type: "draft",
    });
    for (const draft of existing_drafts) {
      const draft_content = draft.content;
      await this.datastore.datastore.delete(draft.getId());
      await this.onReceiveBlockDraft(draft_content);
    }

    let keep_waiting_for_acks = this.latest_seen_at_round ? false : true;
    let keep_waiting_for_certs = true;
    const keep_waiting_interval = setInterval(() => {
      console.log("KEEP_WAITING", {
        time: new Date(),
        round,
        keep_waiting_for_acks,
        keep_waiting_for_certs,
      });
    }, KEEP_WAITING_LOG_INTERVAL_MS);
    const keep_waiting_for_acks_rebroadcast_interval = setInterval(async () => {
      if (keep_waiting_for_acks) {
        console.log(`REBROADCASTING DRAFT for /round/${round}/peer/${this.peerid}`);
        const block_data = await block.toDraftJSONObject();
        await this.communication.broadcastDraftBlock({
          from: this.peerid,
          block_data: block_data,
        });
      }
    }, KEEP_WAITING_FOR_ACKS_REBROADCAST_INTERVAL_MS);
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
        // console.log(this.peerid, {valid_acks});
        if (valid_acks >= current_round_threshold) {
          await block.generateCert(this.keypair);
          await block.save({ datastore: this.datastore });
          if (this.communication) {
            await this.communication.broadcastCertifiedBlock({
              from: this.peerid,
              block_data: await block.toJSONObject(),
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
    clearInterval(keep_waiting_interval);
    clearInterval(keep_waiting_for_acks_rebroadcast_interval);
    await this.bumpCurrentRound();
  }

  async onFetchScribeRoundCertifiedBlockRequest({ round, scribe }) {
    return this.datastore.findBlock({ round, scribe });
  }

  async requestRoundDataFromPeers(round) {
    const scribes = await this.getScribesAtRound(round);
    for (const scribe of scribes) {
      const block = await this.communication.fetchScribeRoundCertifiedBlock({
        from: this.peerid,
        to: scribe,
        scribe,
        round,
      });
      if (block) {
        await this.onReceiveBlockCert(block);
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
    // const round = Round.from({ round_id: round_num });
    // round.scribes = await this.getScribesAtRound(round_num);
    // await round.save({ datastore: this.datastore });
    await this.datastore.setCurrentRound(round_num);
  }

  async bumpCurrentRound() {
    const round_num = await this.datastore.getCurrentRound();
    // const round = Round.from({ round_id: round_num });
    // round.scribes = await this.getScribesAtRound(round_num);
    // await round.save({ datastore: this.datastore });
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

  async run(signal, { beforeEachRound, afterEachRound }) {
    // eslint-disable-next-line no-constant-condition
    while (true) {
      if (beforeEachRound) {
        await beforeEachRound();
      }
      await this.runRound(signal);
      if (afterEachRound) {
        await afterEachRound();
      }
    }
  }
}
