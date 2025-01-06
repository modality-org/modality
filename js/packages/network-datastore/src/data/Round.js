import Model from "./Model.js";
export default class Round extends Model {
  static id_path = "/consensus/round/${round}";
  static fields = ["round", "scribes"];
  static field_defaults = {
    scribes: [],
  };

  static findMaxId({ datastore }) {
    return datastore.findMaxIntKey(`/consensus/round`);
  }

  addScribe(scribe_peer_id) {
    this.scribes.push(scribe_peer_id);
  }

  removeScribe(scribe_peer_id) {
    this.scribes = this.scribes.filter((s) => s !== scribe_peer_id);
  }
}
