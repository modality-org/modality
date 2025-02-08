import Model from "./Model.js";

import Block from "./Block.js";

export default class BlockHeader extends Model {
  static id_path = "/block_headers/round/${round_id}/peer/${peer_id}";
  static fields = [
    "round_id",
    "peer_id",
    "prev_round_certs",
    "opening_sig",
    "cert",
  ];

  static async findAllInRound({ datastore, round_id }) {
    const prefix = `/block_headers/round/${round_id}/peer`;
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

  static async derviveAllInRound({ datastore, round_id }) {
    const blocks = await Block.findAllInRound({ datastore, round_id });
    for (const block of blocks) {
      const bh = await BlockHeader.findOne({
        datastore,
        round_id,
        peer_id: block.peer_id,
      });
      if (!bh) {
        // check validity
        const bh = BlockHeader.from({
          round_id,
          peer_id: block.peer_id,
          prev_round_certs: block.prev_round_certs,
          opening_sig: block.opening_sig,
          cert: block.cert,
        });
        await bh.save({ datastore });
      }
    }
  }
}
