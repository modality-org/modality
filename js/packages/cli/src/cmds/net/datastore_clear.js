// see yargs docs
export const command = "datastore-clear";
export const describe = "clear the datastore of network node";
export const builder = {
  config: {},
  prefix: {default: ""}
};

import Node from "@modality-dev/network-node/Node";

export async function handler({ config, prefix }) {
  const node = await Node.fromConfigFilepath(config);
  await node.setupAsClient();
  const it = node.getDatastore().iterator({prefix});
  for await (const [key] of it) {
    console.log(`DELETING ${key}`);
    await node.getDatastore().datastore.delete(key);
  }
  await node.stop();
}

export default handler;

// so we can directly test the file
import cliCalls from "cli-calls";
await cliCalls(import.meta, handler);
