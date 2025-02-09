// see yargs docs
export const command = "datastore-get";
export const describe = "list the keys of the datastore of network node";
export const builder = {
  config: {},
  prefix: {
    default: ""
  },
  key: {
  }
};

import Node from "@modality-dev/network-node/Node";

export async function handler({ config, prefix, key }) {
  const node = await Node.fromConfigFilepath(config);
  await node.setupAsClient();
  if (key) {
    const value = await node.getDatastore().getString(key);
    console.log(key);
    console.log(JSON.stringify(JSON.parse(value.toString()), null, 2));
    console.log();
  } else {
    const it = node.getDatastore().iterator({prefix});
    for await (const [key, value] of it) {
      console.log(key);
      console.log(JSON.stringify(JSON.parse(value.toString()), null, 2));
      console.log();
    }
  }

  await node.stop();
}

export default handler;

// so we can directly test the file
import cliCalls from "cli-calls";
await cliCalls(import.meta, handler);
