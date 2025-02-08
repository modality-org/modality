import { expect, describe, test, it, afterEach } from "@jest/globals";

import NetworkDatastore from "../src/NetworkDatastore";
import fs from 'fs-extra';

import { dirname } from "dirname-filename-esm";
const __dirname = dirname(import.meta);
const FIXTURES_COMMON = `${__dirname}/../../../fixtures-common`;

describe("devnet1", () => {
  it("should work", async () => {
    const datastore = await NetworkDatastore.createInMemory();
    const network_config = fs.readJSONSync(`${FIXTURES_COMMON}/network-configs/devnet1/config.json`);
    await datastore.loadNetworkConfig(network_config);
    const round0_blocks = await datastore.getKeys('/blocks/round/0');
    expect(round0_blocks.length).toBe(1);
  });
});
