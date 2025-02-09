// see yargs docs
export const command = "run-node";
export const describe = "Run a Modality Network node";
export const builder = {
  config: {},
  enable_consensus: {}
};

import Node from "@modality-dev/network-node/Node";

export async function handler({ config, enable_consensus }) {
  const node = await Node.fromConfigFilepath(config);
  await node.setupAsServer();
  console.log("Running node as %s", node.peerid);
  console.log("             on %s", node.listeners);

  await new Promise((r) => setTimeout(r, 5 * 1000));

  if (config.enable_consensus || enable_consensus) {
    const consensus = await node.setupLocalConsensus();
    consensus.no_events_round_wait_time_ms = 500;
    const controller = new AbortController();
    process.on("SIGINT", () => {
      controller.abort();
    });
    consensus.run(controller.signal, {
      beforeEachRound: async () => {
        const round = await node.getDatastore().getCurrentRound();
        console.log("running round:", round);
      },
    });
  }
}

export default handler;

// so we can directly test the file
import cliCalls from "cli-calls";
await cliCalls(import.meta, handler);
