import SafeJSON from "@modality-dev/utils/SafeJSON";
import Keypair from "@modality-dev/utils/Keypair";
import Model from "./Model.js";

import * as EncodedText from "@modality-dev/utils/EncodedText";

// Narwhal style vertices
export default class Block extends Model {
  static id_path = "/blocks/round/${round_id}/peer/${peer_id}";
  static fields = [
    "round_id",
    "peer_id",
    "prev_round_certs",
    "opening_sig",
    "events",
    "closing_sig",
    "acks",
    "late_acks",
    "cert",
  ];
  static field_defaults = {
    events: [],
    prev_round_certs: {},
    acks: {},
    late_acks: [],
  };

  static async findAllInRound({ datastore, round_id }) {
    const prefix = `/blocks/round/${round_id}/peer`;
    const it = datastore.iterator({ prefix });
    const r = [];
    for await (const [key, value] of it) {
      const peer_id = key.split(`${prefix}/`)[1];
      const block = await this.findOne({ datastore, round_id, peer_id });
      if (block) {
        r.push(block);
      }
    }
    return r;
  }

  toDraftJSONObject() {
    return {
      peer_id: this.peer_id,
      round_id: this.round_id,
      prev_round_certs: this.prev_round_certs,
      opening_sig: this.opening_sig,
      events: this.events,
      closing_sig: this.closing_sig,
    };
  }

  toDraftJSONString() {
    return JSON.stringify(this.toDraftJSONObject);
  }

  addEvent(event) {
    this.events.push(event);
  }

  setNumber(number) {
    this.number = number;
  }

  async generateOpeningSig(keypair) {
    this.opening_sig = await keypair.signJSON({
      round_id: this.round_id,
      peer_id: this.peer_id,
      prev_round_certs: this.prev_round_certs,
    });
    return this.opening_sig;
  }

  async generateClosingSig(keypair) {
    this.closing_sig = await keypair.signJSON({
      round_id: this.round_id,
      peer_id: this.peer_id,
      prev_round_certs: this.prev_round_certs,
      opening_sig: this.opening_sig,
      events: this.events,
    });
    return this.closing_sig;
  }

  getHash() {
    if (!this.closing_sig) {
      throw new Error("no hash without closing sig");
    }
    const hash = EncodedText.recode(this.closing_sig, "base64pad", "base58btc");
    return hash;
  }

  async generateSig(keypair) {
    await this.generateOpeningSig(keypair);
    return await this.generateClosingSig(keypair);
  }

  validateOpeningSig() {
    const keypair = Keypair.fromPublicKey(this.peer_id);
    return keypair.verifyJSON(this.opening_sig, {
      round_id: this.round_id,
      peer_id: this.peer_id,
      prev_round_certs: this.prev_round_certs,
    });
  }

  validateClosingSig() {
    const keypair = Keypair.fromPublicKey(this.peer_id);
    return keypair.verifyJSON(this.closing_sig, {
      round_id: this.round_id,
      peer_id: this.peer_id,
      prev_round_certs: this.prev_round_certs,
      opening_sig: this.opening_sig,
      events: this.events,
    });
  }

  validateSig() {
    return this.validateClosingSig();
  }

  async generateAck(keypair) {
    const peer_id = await keypair.asPublicAddress();
    const facts = {
      peer_id: this.peer_id,
      round_id: this.round_id,
      closing_sig: this.closing_sig,
      acker: peer_id,
    };
    const acker_sig = await keypair.signJSON(facts);
    return {
      peer_id: this.peer_id,
      round_id: this.round_id,
      closing_sig: this.closing_sig,
      acker: peer_id,
      acker_sig,
    };
  }

  async generateLateAck(keypair, seen_at_round) {
    const peer_id = await keypair.asPublicAddress();
    const facts = {
      peer_id: this.peer_id,
      round_id: this.round_id,
      seen_at_round,
      sig: this.sig,
      acker: peer_id,
    };
    const acker_sig = await keypair.signJSON(facts);
    return {
      peer_id: this.peer_id,
      round_id: this.round_id,
      seen_at_round,
      sig: this.sig,
      acker: peer_id,
      acker_sig,
    };
  }

  async validateAck(ack) {
    if (!ack || !ack.acker || !ack.acker_sig) {
      return false;
    }
    const keypair = Keypair.fromPublicKey(ack.acker);
    const facts = {
      peer_id: this.peer_id,
      round_id: this.round_id,
      closing_sig: this.closing_sig,
      acker: ack.acker,
    };
    return await keypair.verifyJSON(ack.acker_sig, facts);
  }

  async addAck(ack) {
    const is_valid = await this.validateAck(ack);
    if (is_valid) {
      this.acks[ack.acker] = ack.acker_sig;
      return true;
    }
  }

  validateAcks() {
    for (const [acker, acker_sig] of Object.entries(this.acks)) {
      const keypair = Keypair.fromPublicKey(acker);
      if (
        !keypair.verifyJSON(acker_sig, {
          round_id: this.round_id,
          peer_id: this.peer_id,
          closing_sig: this.closing_sig,
        })
      ) {
        return false;
      }
    }
    return true;
  }

  countValidAcks() {
    let valid_acks = 0;
    for (const [acker, acker_sig] of Object.entries(this.acks)) {
      const keypair = Keypair.fromPublicKey(acker);
      if (
        keypair.verifyJSON(acker_sig, {
          peer_id: this.peer_id,
          round_id: this.round_id,
          events: this.events,
          sig: this.sig,
        })
      ) {
        valid_acks += 1;
      }
    }
    return valid_acks;
  }

  async validateLateAck(ack) {
    // return true;
    return true;
  }

  async addLateAck(ack) {
    const is_valid = await this.validateAck(ack);
    if (is_valid) {
      this.late_acks.push(ack);
      return true;
    }
  }

  async generateCert(keypair) {
    this.cert = await keypair.signJSON({
      round_id: this.round_id,
      peer_id: this.peer_id,
      prev_round_certs: this.prev_round_certs,
      opening_sig: this.opening_sig,
      events: this.events,
      closing_sig: this.closing_sig,
      acks: this.acks,
    });
    return this.cert;
  }

  async validateCertSig() {
    const keypair = Keypair.fromPublicKey(this.peer_id);
    return keypair.verifyJSON(this.cert, {
      peer_id: this.peer_id,
      round_id: this.round_id,
      prev_round_certs: this.prev_round_certs,
      opening_sig: this.opening_sig,
      events: this.events,
      closing_sig: this.closing_sig,
      acks: this.acks,
    });
  }

  async validateCert({ acks_needed }) {
    const isCertSigValid = await this.validateCertSig();
    if (!isCertSigValid) {
      return false;
    }
    const validAckCount = await this.countValidAcks();
    return validAckCount >= acks_needed;
  }
}
