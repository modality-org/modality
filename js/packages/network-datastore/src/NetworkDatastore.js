import { LevelDatastore } from "datastore-level";
import LevelMem from "level-mem";
import LevelRocksDb from "level-rocksdb";
import SafeJSON from "@modality-dev/utils/SafeJSON";
import fs from "fs";

import Page from './data/Page.js';

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

  async getString(key) {
    return (await this.datastore.get(key)).toString();
  }

  async getJSON(key) {
    return SafeJSON.parse((await this.datastore.get(key)).toString());
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
    const current_round = await this.getDataByKey(
      "/consensus/status/current_round"
    );
    const current_round_num = parseInt(current_round) || 0;
    return this.put(
      "/consensus/status/current_round",
      (current_round_num + 1).toString()
    );
  }

  async setCurrentRound(round) {
    return this.put(
      "/consensus/status/current_round",
      parseInt(round).toString()
    );
  }

  async getCurrentRound() {
    return parseInt(
      (await this.getDataByKey("/consensus/status/current_round"))?.toString()
    );
  }

  async findPage({round, scribe}) {
    return Page.findOne({ datastore: this, round, scribe });
  }

  async doesPageCertLinkToPage(later_page, earlier_page) {
    if (later_page.round <= earlier_page.round) return false;
    let round = later_page.round - 1;
    let cert_set = new Set([
      ...Object.values(later_page.last_round_certs).map((i) => i.scribe),
    ]);
    while (cert_set.size && round >= earlier_page.round) {
      if (round === earlier_page.round && cert_set.has(earlier_page.scribe)) {
        return true;
      }
      const new_cert_set = new Set();
      for (const scribe of cert_set) {
        let page = await Page.findOne({ datastore: this, round, scribe });
        if (!page) {
          throw new Error(
            `Page ${scribe} ${round} not found. You must retrieve it first.`
          );
        }
        for (const i_cert of Object.values(page.last_round_certs)) {
          new_cert_set.add(i_cert.scribe);
        }
      }
      round = round - 1;
      cert_set = new_cert_set;
    }
    return false;
  }

  async findCausallyLinkedPages(last_page, after_page = null) {
    const r = [];
    if (!last_page) return r;
    if (last_page === after_page) return r;
    r.push({ round: last_page.round, scribe: last_page.scribe });
    let page;
    let round = last_page.round - 1;

    // TODO prioritize pages by MIN(ack_count, 2f+1), then by leader-first-lexicographic order,
    // recursively causally order their ack linked pages with the same prioritization strategy.
    // with some binders, this prevents a scribe from silently self-acking as means of prioritizing a commit

    let cert_set = new Set([
      ...Object.values(last_page.last_round_certs).map((i) => i.scribe),
    ]);
    while (cert_set.size && round >= 1) {
      const new_cert_set = new Set();
      // prioritize pages lexographically ordered starting at leader scribe
      const certs_list_lexiordered = [...cert_set].sort();
      const certs_list_start = Math.max(
        0,
        certs_list_lexiordered.findIndex(
          (i) => i.localeCompare(last_page.scribe) > 0
        )
      );
      const certs_list = [
        ...certs_list_lexiordered.slice(certs_list_start),
        ...certs_list_lexiordered.slice(0, certs_list_start),
      ];
      for (const scribe of certs_list) {
        page = await Page.findOne({ datastore: this, round, scribe });
        if (!page) {
          throw new Error(
            `Page ${scribe} ${round} not found. You must retrieve it first.`
          );
        }
        let should_skip = false;
        if (after_page) {
          if (
            page.scribe === after_page.scribe &&
            page.round === after_page.round
          ) {
            should_skip = true;
          } else if (page.round < after_page.round) {
            if (await this.doesPageCertLinkToPage(after_page, page)) {
              // console.log(`
              //   processing ${last_page.round}.${last_page.scribe}
              //     skipping ${page.round}.${page.scribe}
              //     because causally linked to
              //     skipping ${after_page.round}.${after_page.scribe}
              //   `)
              should_skip = true;
            } else {
              //
            }
          }
        }
        if (!should_skip) {
          r.push({ round: page.round, scribe: page.scribe });
          for (const cert of Object.values(page.last_round_certs || {})) {
            new_cert_set.add(cert.scribe);
          }
        } else {
          new_cert_set.delete(page.scribe);
        }
      }

      cert_set = new_cert_set;
      round = round - 1;
    }

    return r.reverse();
  }

  async getTimelyCertsAtRound(round) {
    const pages = (
      await Page.findAllInRound({ datastore: this, round })
    ).filter((i) => !i.seen_at_round);
    return pages.reduce((acc, i) => {
      acc[i.scribe] = i;
      return acc;
    }, {});
  }

  async getTimelyCertSigsAtRound(round) {
    const pages = (
      await Page.findAllInRound({ datastore: this, round })
    ).filter((i) => !i.seen_at_round);
    return pages.reduce((acc, i) => {
      acc[i.scribe] = {
        scribe: i.scribe,
        cert: i.cert,
        round: i.round,
      };
      return acc;
    }, {});
  }
}
