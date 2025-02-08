import { jest, expect, describe, test, it } from "@jest/globals";

import tmp from "tmp";
import path from "path";
import fs from "fs-extra";
import StreamZip from "node-stream-zip";

import { dirname } from "dirname-filename-esm";
const __dirname = dirname(import.meta);

import NetworkDatastore from "../src/NetworkDatastore";

describe("DevnetStatic1", () => {
  it.skip("should work", async () => {
    const fixturesDir = path.resolve(`${__dirname}/../../../fixtures/`);
    const tmpDir = tmp.dirSync({ prefix: "tmpDir" });
    fs.copyFileSync(
      `${fixturesDir}/devnet-static1-datastore.zip`,
      `${tmpDir.name}/devnet-static1-datastore.zip`
    );
    const zip = new StreamZip.async({
      file: `${tmpDir.name}/devnet-static1-datastore.zip`,
    });
    await zip.extract(null, tmpDir.name);

    const datastore = await NetworkDatastore.createInDirectory(
      `${tmpDir.name}/devnet-static1-datastore`
    );
    // TODO
  });
});
