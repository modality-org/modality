import SafeJSON from "@modality-dev/utils/SafeJSON";

export default class Model {
  static id_path = null;
  static fields = [];
  static field_defaults = {};

  static from(obj) { 
    const r = new this(obj);
    for (const key of this.fields) {
      if (obj[key] !== undefined) {
        r[key] = obj[key];
      } else if (this.field_defaults?.[key] !== undefined) {
        const default_value = this.field_defaults[key]; 
        if (Array.isArray(default_value)) {
          r[key] = [...default_value];
        } else if (typeof default_value === 'object') {
          r[key] = {...default_value};
        } else {
          r[key] = default_value;
        }
      }
    }
    return r;
  }

  static fromJSONString(json) {
    if (!json) return null;
    const obj = SafeJSON.parse(json)
    return this.from(obj);
  }

  static fromJSONObject(obj) {
    return this.from(obj);
  }

  toJSON() {
    return this.toJSONString();
  }

  toJSONString() {
    return JSON.stringify(this.toJSONObject());
  }

  toJSONObject() {
    const obj = {};
    for (const key of this.constructor.fields) {
      obj[key] = this[key];
    }
    return obj;
  }

  async save({ datastore }) {
    return datastore.put(this.getId(), this.toJSONString());
  }

  static getIdFor(keys) {
    let id = this.id_path;
    return id.replaceAll(/\${(\w+)}/g, (match, key) => keys[key] ?? match);
  }

  static getKeyNames() {
    const keyPattern = /\$\{(\w+)\}/g;
    const matches = [...this.id_path.matchAll(keyPattern)];
    return matches.map(match => match[1]);
  }

  getIdKeys() {
    const key_names = this.constructor.getKeyNames();
    const keys = {};
    for (const key of key_names) {
      keys[key] = this[key];
    }
    return keys;
  }

  getId() {
    const keys = this.getIdKeys();
    return this.constructor.getIdFor(keys);
  }


  static async findOne({ datastore, ...keys }) {
    const key = this.getIdFor(keys);
    try {
      const v = await datastore.get(key);
      return this.fromJSONString(v.toString());
    } catch (e) {
      if (e.code === "ERR_NOT_FOUND") {
        return null;
      } else {
        throw e;
      }
    }
  }

  async reload({datastore}) {
    const keys = this.getIdKeys();
    const obj = await this.constructor.findOne({ datastore, ...keys });
    for (const key of this.constructor.fields) {
      this[key] = obj[key];
    }
    return this;
  }

  async delete({ datastore }) {
    return datastore.delete(this.getId());
  }

}