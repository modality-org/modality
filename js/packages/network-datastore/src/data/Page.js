import SafeJSON from "@modality-dev/utils/SafeJSON";
import Keypair from "@modality-dev/utils/Keypair";
import Model from './Model.js';

// Narwhal style vertices
export default class Page extends Model {
  static id_path = "/consensus/round/${round}/scribe/${scribe}";
  static fields = [
    "scribe",
    "round",
    "last_round_certs",
    "events",
    "hash",
    "sig",
    "acks",
    "late_acks",
    "cert",
    "is_section_leader",
    "section_ending_round",
    "section_starting_round",
    "section_page_number",
    "page_number",
    "seen_at_round",
  ];
  static field_defaults = {
    events: [],
    last_round_certs: {},
    acks: {},
    late_acks: [],
  }

  static async findAllInRound({ datastore, round }) {
    const prefix = `/consensus/round/${round}/scribe`;
    const it = datastore.iterator({ prefix });
    const r = [];
    for await (const [key, value] of it) {
      const scribe = key.split(`${prefix}/`)[1];
      const page = await this.findOne({ datastore, round, scribe });
      if (page) {
        r.push(page);
      }
    }
    return r;
  }

  toDraftJSONObject() {
    return {
      scribe: this.scribe,
      round: this.round,
      last_round_certs: this.last_round_certs,
      events: this.events,
      sig: this.sig,
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

  async generateSig(keypair) {
    this.sig = await keypair.signJSON({
      scribe: this.scribe,
      round: this.round,
      last_round_certs: this.last_round_certs,
      events: this.events,
    });
    return this.sig;
  }

  validateSig() {
    const keypair = Keypair.fromPublicKey(this.scribe);
    return keypair.verifyJSON(this.sig, {
      scribe: this.scribe,
      round: this.round,
      last_round_certs: this.last_round_certs,
      events: this.events,
    });
  }

  async generateAck(keypair) {
    const peer_id = await keypair.asPublicAddress();
    const facts = {
      scribe: this.scribe,
      round: this.round,
      sig: this.sig,
    };
    const acker_sig = await keypair.signJSON(facts);
    return {
      scribe: this.scribe,
      round: this.round,
      sig: this.sig,
      acker: peer_id,
      acker_sig,
    };
  }

  async generateLateAck(keypair, seen_at_round) {
    const peer_id = await keypair.asPublicAddress();
    const facts = {
      scribe: this.scribe,
      round: this.round,
      seen_at_round,
      sig: this.sig,
    };
    const acker_sig = await keypair.signJSON(facts);
    return {
      scribe: this.scribe,
      round: this.round,
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
      scribe: this.scribe,
      round: this.round,
      sig: this.sig,
    };
    return await keypair.verifyJSON(ack.acker_sig, facts);
  }

  async addAck(ack) {
    const is_valid = await this.validateAck(ack);
    if (is_valid) {
      this.acks[ack.acker] = ack;
      return true;
    }
  }

  validateAcks() {
    for (const ack of Object.values(this.acks)) {
      const keypair = Keypair.fromPublicKey(ack.acker);
      if (
        !keypair.verifyJSON(ack.acker_sig, {
          scribe: this.scribe,
          round: this.round,
          last_round_certs: this.last_round_certs,
          events: this.events,
          sig: this.sig,
        })
      ) {
        return false;
      }
    }
    return true;
  }

  countValidAcks() {
    let valid_acks = 0;
    for (const ack of Object.values(this.acks)) {
      const keypair = Keypair.fromPublicKey(ack.acker);
      if (
        keypair.verifyJSON(ack.acker_sig, {
          scribe: this.scribe,
          round: this.round,
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
      scribe: this.scribe,
      round: this.round,
      last_round_certs: this.last_round_certs,
      events: this.events,
      acks: this.acks,
    });
    return this.cert;
  }

  async validateCertSig() {
    const keypair = Keypair.fromPublicKey(this.scribe);
    return keypair.verifyJSON(this.cert, {
      scribe: this.scribe,
      round: this.round,
      last_round_certs: this.last_round_certs,
      events: this.events,
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
