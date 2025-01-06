import Model from "./Model.js";
export default class ContractCommitEvent extends Model {
  static id_path = "/consensus/contract_commit_event/${timestamp}/${contract_id}/${commit_id}";
  static fields = ["timestamp", "contract_id", "commit_id", "data"];
  static field_defaults = {
    scribes: [],
  };

  static async findAll({ datastore }) {
    const prefix = `/consensus/contract_commit_event/`;
    const it = datastore.iterator({ prefix });
    const r = [];
    for await (const [key, value] of it) {
      const timestamp = key.split(`${prefix}/`)[1];
      const contract_id = key.split(`${prefix}/`)[2];
      const commit_id = key.split(`${prefix}/`)[3];
      const ContractCommitEvent = await this.findOne({ datastore, timestamp, contract_id, commit_id });
      if (ContractCommitEvent) {
        r.push(ContractCommitEvent);
      }
    }
    return r;
  }

  static async saveEvent({ timestamp, contract_id, commit_id, data }) {
    const datastore = Model.from({ timestamp, contract_id, commit_id, data });
    await this.save({ datastore });
  }
}
