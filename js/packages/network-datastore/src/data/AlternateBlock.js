import Model from './Model.js';

export default class AlternateBlock extends Model {
  static id_path = "/alternates/blocks/round/${round_id}/peer/${peer_id}/hash/${hash}";
  static fields = [
    "round_id",
    "peer_id",
    "prev_round_certs",
    "opening_sig",
    "events",
    "closing_sig",
    "hash",
    "acks",
    "late_acks",
    "cert",
  ];
  static field_defaults = {
    events: [],
    prev_round_certs: {},
    acks: {},
    late_acks: [],
  }

  static async findAllInRound({ datastore, round_id }) {
    const prefix = `/alternates/blocks/round/${round_id}`;
    const it = datastore.iterator({ prefix });
    const r = [];
    for await (const [key, value] of it) {
      const matcher = key.match(new RegExp(`${prefix}/peer/([a-z0-9])+/hash/([a-z0-9])+`));
      const peer_id = matcher[1];
      const hash = matcher[2];
      const block = await this.findOne({ datastore, round_id, peer_id, hash });
      if (block) {
        r.push(block);
      }
    }
    return r;
  }
}
