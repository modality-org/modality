import Page from "@modality-dev/network-datastore/data/Page";
import Round from "@modality-dev/network-datastore/data/Round";
import RoundMessage from "@modality-dev/network-datastore/data/RoundMessage";
import ContractCommitEvent from "@modality-dev/network-datastore/data/ContractCommitEvent";
import ConsensusMath from "./lib/ConsensusMath";

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

  async getScribesAtRound(round) {
    return this.sequencing.getScribesAtRound(round);
  }

  async consensusThresholdForRound(round) {
    return this.sequencing.consensusThresholdForRound(round);
  }

  async onReceiveDraftPage(page_data) {
    const page = await Page.fromJSONObject(page_data);
    if (!page.validateSig()) {
      console.warn("invalid sig");
      return;
    }

    const round_scribes = await this.getScribesAtRound(page.round);
    if (!round_scribes.includes(page.scribe)) {
      console.warn(
        `ignoring non-scribe ${page.scribe} at round ${page.round}`
      );
      return;
    }

    const current_round = await this.datastore.getCurrentRound();

    if (page.round > current_round) {
      return this.onReceiveDraftPageFromLaterRound(page_data);
    } else if (page.round < current_round) {
      return this.onReceiveDraftPageFromEarlierRound(page_data);
    } else {
      return this.onReceiveDraftPageFromCurrentRound(page_data);
    }
  }

  async onReceiveDraftPageFromEarlierRound(page_data) {
    const current_round = await this.datastore.getCurrentRound();
    const page = await Page.fromJSONObject(page_data);
    // console.warn(`received draft for earlier round: round ${page.round} draft received but currently on round ${current_round}`);

    // TODO provide same late ack if asked again

    // provide late ack
    if (this.peerid) {
      const ack = await page.generateLateAck(this.keypair, current_round);
      if (this.communication) {
        const last_round_certs = await this.datastore.getTimelyCertSigsAtRound(
          current_round - 1
        );
        await this.communication.sendPageLateAck({
          from: this.peerid,
          to: ack.scribe,
          ack_data: ack,
          extra: { last_round_certs },
        });
      }
      return ack;
    }
  }

  async onReceiveDraftPageFromLaterRound(page_data) {
    const current_round = await this.datastore.getCurrentRound();
    const page = await Page.fromJSONObject(page_data);
    // console.warn(`received draft for later round: round ${page.round} draft received but currently on round ${current_round}`);

    await RoundMessage.fromJSONObject({
      round: page.round,
      scribe: page.scribe,
      type: "draft",
      seen_at_round: current_round,
      content: page_data,
    }).save({ datastore: this.datastore });

    // TODO considering bumping rounds!
    // TODO req and verify acker's last_round_certs chain
    if (current_round < page.round) {
      if (
        !this.latest_seen_at_round ||
        page_data.round > this.latest_seen_at_round
      ) {
        this.latest_seen_at_round = page_data.round;
        return;
      }
    }
  }

  async onReceiveDraftPageFromCurrentRound(page_data) {
    const current_round = await this.datastore.getCurrentRound();
    const page = await Page.fromJSONObject(page_data);

    if (this.peerid) {
      const ack = await page.generateAck(this.keypair);
      if (this.communication) {
        await this.communication.sendPageAck({
          from: this.peerid,
          to: ack.scribe,
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

    const whoami = await this.keypair?.asPublicAddress();
    if (!whoami || whoami !== ack.scribe) {
      return;
    }

    const round = await this.datastore.getCurrentRound();
    if (ack.round !== round) {
      return;
    }

    const round_scribes = await this.getScribesAtRound(ack.acker);
    if (!round_scribes.includes(ack.acker)) {
      console.warn(
        `ignoring non-scribe ack ${ack.acker} at round ${ack.round}`
      );
      return;
    }

    const page = await Page.findOne({
      datastore: this.datastore,
      round,
      scribe: whoami,
    });
    if (page) {
      await this.mutex.runExclusive(async () => {
        await page.addAck(ack);
        await page.save({ datastore: this.datastore });
      });
    }
  }

  async onReceivePageLateAck(ack) {
    return;
  }

  async onReceiveCertifiedPage(page_data) {
    const page = await Page.fromJSONObject(page_data);
    if (!page.validateSig()) {
      return null;
    }

    const round = await this.datastore.getCurrentRound();
    if (page.round < round) {
      // console.log({round}, page_data);
      // return this.onReceiveLateCertifiedPage(page_data);
    } else if (page.round > round) {
      return this.onReceiveCertifiedPageFromLaterRound(page_data);
    }

    return this.onReceiveCertifiedPageFromCurrentRound(page_data);
  }

  async onReceiveCertifiedPageFromLaterRound(page_data) {
    const current_round = await this.datastore.getCurrentRound();
    const page = await Page.fromJSONObject(page_data);

    await RoundMessage.fromJSONObject({
      round: page.round,
      scribe: page.scribe,
      type: "certified",
      seen_at_round: current_round,
      content: page_data,
    }).save({ datastore: this.datastore });

    // TODO considering bumping rounds!
    // TODO req and verify acker's last_round_certs chain
    if (current_round < page.round) {
      if (
        !this.latest_seen_at_round ||
        page_data.round > this.latest_seen_at_round
      ) {
        this.latest_seen_at_round = page_data.round;
        return;
      }
    }
  }

  async onReceiveCertifiedPageFromCurrentRound(page_data) {
    const page = await Page.fromJSONObject(page_data);
    if (!page.validateSig()) {
      return null;
    }
    const round = page.round;

    const last_round_threshold = await this.consensusThresholdForRound(
      round - 1
    );
    const current_round_threshold =
      await this.consensusThresholdForRound(round);

    if (
      round > 1 &&
      Object.keys(page.last_round_certs).length < last_round_threshold
    ) {
      return null;
    }

    const has_valid_cert = await page.validateCert({
      acks_needed: current_round_threshold,
    });
    if (!has_valid_cert) {
      return null;
    }

    await page.save({ datastore: this.datastore });
    return page;
  }

  async getOrFetchLastRoundCerts(round) {
    const last_round = round - 1;
    let last_round_certs = await this.datastore.getTimelyCertSigsAtRound(last_round);
    const last_round_scribes = await this.getScribesAtRound(last_round);

    const threshold = ConsensusMath.calculate2fplus1(
      last_round_scribes.length
    );
    if (Object.keys(last_round_certs) >= threshold) {
      return last_round_certs;
    }

    if (this.communication) {
      for (const scribe of last_round_scribes) {
        const page_data =
          await this.communication.fetchScribeRoundCertifiedPage({
            from: this.peerid,
            to: scribe,
            scribe,
            round: last_round,
          });
        if (page_data) {
          const page = await Page.fromJSONObject(page_data);
          if (page.validateCert({ acks_needed: threshold })) {
            await page.save({ datastore: this.datastore });
          }
        }
      }
    }

    last_round_certs = await this.datastore.getTimelyCertSigsAtRound(last_round);

    return last_round_certs;
  }



  async speedUpToLatestUncertifiedRound() {
    let round_certified = true;
    while (round_certified) {
      let round = await this.datastore.getCurrentRound() + 1;
      const last_round_certs = await this.getOrFetchLastRoundCerts(round);
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
      const cert_count = Object.keys(last_round_certs).length;
      if (cert_count && threshold && cert_count >= threshold) {
        await this.bumpCurrentRound(); 
      } else {
        round_certified = false;
      }
    }
  }

  async runRound(signal) {
    const scribe = await this.keypair?.asPublicAddress();

    await this.speedUpToLatestUncertifiedRound();
    let round = await this.datastore.getCurrentRound();

    const last_round_certs = await this.getOrFetchLastRoundCerts(round);
    const last_round_scribes = await this.getScribesAtRound(round - 1);

    const threshold = await this.consensusThresholdForRound(round - 1);
    const cert_count = Object.keys(last_round_certs).length;
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
    const page = Page.from({
      round,
      scribe,
      last_round_certs,
      events,
    });
    await page.generateSig(this.keypair);
    await page.save({ datastore: this.datastore });

    if (this.communication) {
      const page_data = await page.toDraftJSONObject();
      await this.communication.broadcastDraftPage({
        from: this.peerid,
        page_data,
      });
    }

    // handle enqueue round messages
    const existing_drafts = await RoundMessage.findAllInRoundOfType({
      datastore: this.datastore,
      round,
      type: "draft",
    });
    for (const draft of existing_drafts) {
      // console.log("existing draft", draft);
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
        await page.reload({ datastore: this.datastore });
        const valid_acks = await page.countValidAcks();
        if (valid_acks >= current_round_threshold) {
          await page.generateCert(this.keypair);
          if (this.communication) {
            await this.communication.broadcastCertifiedPage({
              from: this.peerid,
              page_data: await page.toJSONObject(),
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
      // const roundi = Round.from({ round: i });
      // roundi.scribes = await this.getScribesAtRound(i);
      // await roundi.save({ datastore: this.datastore });
    }
    const round = Round.from({ round: round_num });
    round.scribes = await this.getScribesAtRound(round_num);
    await round.save({ datastore: this.datastore });
    await this.datastore.setCurrentRound(round_num);
  }

  async bumpCurrentRound() {
    const round_num = await this.datastore.getCurrentRound();
    const round = Round.from({ round: round_num });
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
