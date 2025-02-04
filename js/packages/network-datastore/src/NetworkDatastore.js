import { LevelDatastore } from "datastore-level";
import LevelMem from "level-mem";
import LevelRocksDb from "level-rocksdb";
import SafeJSON from "@modality-dev/utils/SafeJSON";
import fs from "fs-extra";

import Block from './data/Block.js';
import Round from './data/Round.js';
import RoundBlockHeader from "./data/RoundBlockHeader.js";

export default class NetworkDatastore {
  constructor(datastore) {
    this.datastore = datastore;
    return this;
  }

  static async createWith({ storage_type, storage_path }) {
    if (storage_type === "directory" && storage_path) {
      return this.createInDirectory(storage_path);
    } else {
      return this.createInMemory();
    }
  }

  static async createInMemory() {
    const datastore = new LevelDatastore(`:memory:`, {
      db: LevelMem,
    });
    await datastore.open();
    return new NetworkDatastore(datastore);
  }

  static async createInDirectory(path) {
    await fs.ensureDir(path);
    const datastore = new LevelDatastore(path, {
      db: LevelRocksDb,
    });
    await datastore.open();
    return new NetworkDatastore(datastore);
  }

  async writeToDirectory(path) {
    const datastore = new LevelDatastore(path, {
      db: LevelRocksDb,
    });
    await datastore.open();
    const it = await this.iterator({ prefix: "" });
    for await (const [key, value] of it) {
      await datastore.put(key, value);
    }
  }

  async loadNetworkConfig(network_config) {
    if (network_config?.rounds) {
      for (const [round_id,round_data] of Object.entries(network_config.rounds)) {
        for (const block_data of Object.values(round_data)) {
          const block = Block.from(block_data);
          await block.save({datastore: this});
          const rbh = RoundBlockHeader.from({...block_data});
          await rbh.save({datastore: this});
        }
        const round = Round.from({round_id});
        await round.save({datastore: this});
      }
    }
  }

  async writeToSqlExport(path) {
    const f = fs.createWriteStream(path);
    f.write(
      "CREATE TABLE IF NOT EXISTS key_values (key TEXT PRIMARY KEY, value JSONB); \n"
    );
    const it = await this.iterator({ prefix: "" });
    for await (const [key, value] of it) {
      const escapedKey = key?.replace(/'/g, "''");
      const escapedValue = value.toString().replace(/'/g, "''");
      f.write(
        `INSERT INTO key_values (key, value) VALUES ('${escapedKey}', '${escapedValue}') ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value;\n`
      );
    }
    f.end();
  }

  async cloneToMemory() {
    const datastore = await NetworkDatastore.createInMemory();
    const it = await this.iterator({ prefix: "" });
    for await (const [key, value] of it) {
      await datastore.put(key, value);
    }
    return datastore;
  }

  async getDataByKey(key) {
    try {
      return await this.datastore.get(key);
    } catch (e) {
      if (e.code !== "ERR_NOT_FOUND") {
        throw e;
      }
    }
  }

  async setDataByKey(key, value) {
    await this.datastore.put(key, value.toString());
  }

  get(key) {
    return this.datastore.get(key);
  }

  async getKeys(prefix = "") {
    const it = await this.iterator({ prefix });
    const r = [];
    for await (const [key, value] of it) {
      r.push(key);
    }
    return r;
  }

  async getString(key) {
    return (await this.getDataByKey(key))?.toString();
  }

  async getInt(key) {
    const v = (await this.getDataByKey(key))?.toString();
    if (v) {
      return parseInt(v);
    } else {
      null;
    }
  }

  async getJSON(key) {
    return SafeJSON.parse((await this.getDataByKey(key))?.toString());
  }

  put(key, value) {
    return this.datastore.put(key, value);
  }

  queryKeys(opts) {
    return this.datastore.queryKeys(opts);
  }

  iterator({ prefix, filters, orders }) {
    return this.datastore.db.iterator({
      gt: `${prefix}/`,
      lt: `${prefix}0`,
      filters,
      orders,
    });
  }

  async findMaxStringKey(prefix) {
    const it = this.datastore.db.iterator({
      gt: `${prefix}/`,
      lt: `${prefix}0`,
      reverse: true,
      limit: 1,
    });
    for await (const [key, value] of it) {
      return key.split(`${prefix}/`)[1];
    }
  }

  async findMaxIntKey(prefix) {
    let r = null;
    const it = this.datastore.db.iterator({
      gt: `${prefix}/`,
      lt: `${prefix}0`,
    });
    for await (const [key, value] of it) {
      // TODO safer
      const v = parseInt(key.split(`${prefix}/`)[1]);
      if (r === null) {
        r = v;
      } else if (v > r) {
        r = v;
      }
    }
    return r;
  }

  async bumpCurrentRound() {
    const current_round = await this.getInt(
      "/status/current_round"
    );
    const current_round_num = current_round || 0;
    return this.put(
      "/status/current_round",
      (current_round_num + 1).toString()
    );
  }

  async setCurrentRound(round) {
    return this.put(
      "/status/current_round",
      parseInt(round).toString()
    );
  }

  async getCurrentRound() {
    return this.getInt("/status/current_round");
  }

  async getStatus() {
    const current_round = await this.getInt("/status/current_round");
    return {
      current_round,
    }
  }

  async findBlock({round, scribe}) {
    return Block.findOne({ datastore: this, round, scribe });
  }

  async doesBlockCertLinkToBlock(later_block, earlier_block) {
    if (later_block.round <= earlier_block.round) return false;
    let round = later_block.round - 1;
    let cert_set = new Set([
      ...Object.values(later_block.last_round_certs).map((i) => i.scribe),
    ]);
    while (cert_set.size && round >= earlier_block.round) {
      if (round === earlier_block.round && cert_set.has(earlier_block.scribe)) {
        return true;
      }
      const new_cert_set = new Set();
      for (const scribe of cert_set) {
        let block = await Block.findOne({ datastore: this, round, scribe });
        if (!block) {
          throw new Error(
            `Block ${scribe} ${round} not found. You must retrieve it first.`
          );
        }
        for (const i_cert of Object.values(block.last_round_certs)) {
          new_cert_set.add(i_cert.scribe);
        }
      }
      round = round - 1;
      cert_set = new_cert_set;
    }
    return false;
  }

  async findCausallyLinkedBlocks(last_block, after_block = null) {
    const r = [];
    if (!last_block) return r;
    if (last_block === after_block) return r;
    r.push({ round: last_block.round, scribe: last_block.scribe });
    let block;
    let round = last_block.round - 1;

    // TODO prioritize blocks by MIN(ack_count, 2f+1), then by leader-first-lexicographic order,
    // recursively causally order their ack linked blocks with the same prioritization strategy.
    // with some binders, this prevents a scribe from silently self-acking as means of prioritizing a commit

    let cert_set = new Set([
      ...Object.values(last_block.last_round_certs).map((i) => i.scribe),
    ]);
    while (cert_set.size && round >= 1) {
      const new_cert_set = new Set();
      // prioritize blocks lexographically ordered starting at leader scribe
      const certs_list_lexiordered = [...cert_set].sort();
      const certs_list_start = Math.max(
        0,
        certs_list_lexiordered.findIndex(
          (i) => i.localeCompare(last_block.scribe) > 0
        )
      );
      const certs_list = [
        ...certs_list_lexiordered.slice(certs_list_start),
        ...certs_list_lexiordered.slice(0, certs_list_start),
      ];
      for (const scribe of certs_list) {
        block = await Block.findOne({ datastore: this, round, scribe });
        if (!block) {
          throw new Error(
            `Block ${scribe} ${round} not found. You must retrieve it first.`
          );
        }
        let should_skip = false;
        if (after_block) {
          if (
            block.scribe === after_block.scribe &&
            block.round === after_block.round
          ) {
            should_skip = true;
          } else if (block.round < after_block.round) {
            if (await this.doesBlockCertLinkToBlock(after_block, block)) {
              // console.log(`
              //   processing ${last_block.round}.${last_block.scribe}
              //     skipping ${block.round}.${block.scribe}
              //     because causally linked to
              //     skipping ${after_block.round}.${after_block.scribe}
              //   `)
              should_skip = true;
            } else {
              //
            }
          }
        }
        if (!should_skip) {
          r.push({ round: block.round, scribe: block.scribe });
          for (const cert of Object.values(block.last_round_certs || {})) {
            new_cert_set.add(cert.scribe);
          }
        } else {
          new_cert_set.delete(block.scribe);
        }
      }

      cert_set = new_cert_set;
      round = round - 1;
    }

    return r.reverse();
  }

  async getTimelyCertsAtRound(round_id) {
    const blocks = (
      await Block.findAllInRound({ datastore: this, round_id })
    ).filter((i) => !i.seen_at_round);
    return blocks.reduce((acc, i) => {
      acc[i.peer_id] = i.cert;
      return acc;
    }, {});
  }

  async getTimelyCertSigsAtRound(round_id) {
    const r = await Block.findAllInRound({ datastore: this, round_id })
    const blocks = (
      await Block.findAllInRound({ datastore: this, round_id })
    ).filter((i) => !i.seen_at_round);
    return blocks.reduce((acc, i) => {
      acc[i.peer_id] = {
        cert: i.cert,
        round_id: i.round_id,
        peer_id: i.peer_id,
      };
      return acc;
    }, {});
  }
}
