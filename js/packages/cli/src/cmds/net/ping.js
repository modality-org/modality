// see yargs docs
export const command = 'ping';
export const describe = 'Ping a Modality Network node';
export const builder = {
  config: {
  },
  target: {
  },
  times: {
    type: 'number',
    default: 1
  }
};

import Node from "@modality-dev/network-node/Node";
import { multiaddr } from "@multiformats/multiaddr";

export async function handler({config, target, times = 1}) {
  const node = await Node.fromConfigFilepath(config);
  await node.setupAsClient();

  const random_hex = generateRandomHexString();
  const startTime = Date.now();
  for (let i = 0; i < times; i++) {
    await node.swarm.services.reqres.call(multiaddr(target), "/ping", JSON.stringify({random_hex}));
  }
  const duration = Date.now() - startTime;
  console.log("Time taken to ping %s times: %s", times, duration);
  console.log("Average time taken to ping: %s", duration / times);

  await node.stop();
}

export default handler;

// so we can directly test the file
import cliCalls from "cli-calls";
await cliCalls(import.meta, handler);


function generateRandomHexString() {
  const bytes = new Uint8Array(32);
  crypto.getRandomValues(bytes);
  return Array.from(bytes)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}