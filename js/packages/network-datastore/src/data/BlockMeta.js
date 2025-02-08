import Model from "./Model.js";

import Block from "./Block.js";

export default class BlockMeta extends Model {
  static id_path = "/block_metas/round/${round_id}/peer/${peer_id}";
  static fields = [
    "round_id",
    "peer_id",
    "seen_at_round",
    "is_section_leader",
    "section_ending_round",
    "section_starting_round",
    "section_block_number",
    "block_number",
  ];

  static async findAllInRound({ datastore, round_id }) {
    const prefix = `/block_metas/round/${round_id}/peer`;
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
}
