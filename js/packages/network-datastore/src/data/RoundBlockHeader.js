import Model from "./Model.js";

import Block from './Block.js';

export default class RoundBlockHeader extends Model {
  static id_path = "/round/${round_id}/block_header/${peer_id}";
  static fields = [
    "round_id",
    "peer_id",
    "prev_block_certs",
    "opening_sig",
    "cert"
  ];

  static async findAllInRound({ datastore, round_id }) {
    const prefix = `/round/${round_id}/block_header`;
    const it = datastore.iterator({ prefix });
    const r = [];
    for await (const [key, value] of it) {
      const peer_id = key.split(`${prefix}/`)[1];
      const msg = await this.findOne({ datastore, round_id, peer_id });
      if (msg) {
        r.push(msg);
      }
    }
    return r;
  }

  static async ensureAllInRound({ datastore, round_id }) {
    const blocks = await Block.findAllInRound({datastore, round_id})
    for (const block of blocks) {
      const rbh = await RoundBlockHeader.findOne({datastore, round_id, peer_id: block.peer_id});
      if (!rbh) {
        const rbh = RoundBlockHeader.from({
          round_id,
          peer_id: block.peer_id,
          prev_round_certs: block.prev_round_certs,
          opening_sig: block.opening_sig,
          cert: block.cert,
        });
        await rbh.save({datastore});
      }
    }
  }
}
