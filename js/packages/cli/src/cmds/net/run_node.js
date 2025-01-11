// see yargs docs
export const command = 'run-node';
export const describe = 'Run a Modality Network node';
export const builder = {
  config: {
  },
};

import Node from "@modality-dev/network-node/Node";

export async function handler({config}) {
  const node = await Node.fromConfigFilepath(config);
  await node.setupAsServer();
  console.log("Running node as %s", node.peerid);
  console.log("             on %s", node.listeners);
}

export default handler;

// so we can directly test the file
import cliCalls from "cli-calls";
await cliCalls(import.meta, handler);