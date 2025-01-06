export const command = "create-keypair-file";
export const describe = "create new cryptographic keypairs";

export const builder = {
  dir: {
    required: true,
    desc: "directory",
  },
  number: {
    desc: "number of keypairs to generate",
  },
};

import fs from "fs";
import Keypair from "@modality-dev/utils/Keypair";
import JSONFile from "@modality-dev/utils/JSONFile";
import JSONStringifyDeterministic from "json-stringify-deterministic";

const log = console.log;

export async function handler({
  dir,
  number = 1,
  overwrite = false,
  summarize,
}) {
  const summary = {};
  for (let i = 0; i < number; i++) {
    const key = await Keypair.generate();
    const name = await key.publicKeyAsBase58Identity();
    const base_path = `${dir}/${name}`;
    const path_to_keypair = `${base_path}/signing.keypair`;
    if (fs.existsSync(path_to_keypair)) {
      if (!overwrite) {
        console.error(`ERROR: "${name}" identity already exists`);
        return;
      } else {
        console.log(`WARNING: "${name}" identity already exists, overwriting`);
      }
    }
    if (summarize) {
      summary[name] = await key.asJSON();
    }
    fs.mkdirSync(base_path, { recursive: true });
    await key.asJSONFile(`${base_path}/signing.keypair`);
    log(`crypto:/${await key.asPublicMultiaddress()}`);
  }
  if (summarize) {
    const str = JSONStringifyDeterministic(summary);
    JSONFile.writeSync(`${dir}/summary.json`, JSON.parse(str));
  }
}

export default handler;

import cliCalls from "cli-calls";
await cliCalls(import.meta, handler);
