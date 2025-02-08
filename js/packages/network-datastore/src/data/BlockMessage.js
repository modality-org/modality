import Model from "./Model.js";

export default class BlockMessage extends Model {
  static id_path = "/block_messages/${round}/type/${type}/scribe/${scribe}";
  static fields = ["round", "scribe", "type", "seen_at_round", "content"];

  static async findAllInRoundOfType({ datastore, round, type }) {
    const prefix = `/block_messages/${round}/type/${type}/scribe`;
    const it = datastore.iterator({ prefix });
    const r = [];
    for await (const [key, value] of it) {
      const scribe = key.split(`${prefix}/`)[1];
      const msg = await this.findOne({ datastore, round, type, scribe });
      if (msg) {
        r.push(msg);
      }
    }
    return r;
  }
}
