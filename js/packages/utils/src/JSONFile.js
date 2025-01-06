import fs from "fs";

export default class JSONFile {
  /**
   *
   * @param {string} path
   * @returns {string}
   */
  static readSync(path) {
    if (!path) {
      return null;
    }
    const buf = fs.readFileSync(path, "utf-8");
    return JSON.parse(buf);
  }

  /**
   *
   * @param {string} path
   * @param {string} defaultValue
   * @returns {string}
   */
  static readSyncOrReturnDefault(path, defaultValue) {
    try {
      return JSONFile.readSync(path);
    } catch (e) {
      return defaultValue;
    }
  }

  /**
   *
   * @param {string} path
   * @param {JSON} json
   */
  static writeSync(path, json) {
    const json_string = JSON.stringify(json);
    fs.writeFileSync(path, json_string, "utf-8");
  }

  /**
   *
   * @param {string} path
   * @param {JSON} patch
   */
  static patchObjectSync(path, patch) {
    const original_json = JSONFile.readSync(path);
    const json = {
      ...original_json,
      ...patch,
    };
    JSONFile.writeSync(path, json);
  }

  /**
   *
   * @param {string} path
   * @param {object} patch
   */
  static writeOrPatchObjectSync(path, patch) {
    if (!fs.existsSync(path)) {
      JSONFile.writeSync(path, patch);
    } else {
      JSONFile.patchObjectSync(path, patch);
    }
  }
}
