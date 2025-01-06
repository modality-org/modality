export const command = "create-mock-datastore";
export const describe = "creates a mock datastore";

export const builder = {
  dir: {
    required: true,
    desc: "path to output",
  },
  scribes: {
    desc: "scribe count",
  },
  rounds: {
    desc: "scribe rounds",
  },
};

const log = console.log;

import NetworkDatastoreBuilder from "@modality-dev/network-datastore/NetworkDatastoreBuilder";
import RoundRobin from "@modality-dev/network-consensus/election/RoundRobin";
import DAGRider from "@modality-dev/network-consensus/sequencing/DAGRider";

export async function handler({
  dir = "./tmp/datastore",
  scribes = 5,
  rounds = 12,
}) {
  const builder = await NetworkDatastoreBuilder.createInDirectory(dir);
  await builder.generateScribes(scribes, true);
  await builder.addFullyConnectedRound();
  for (let i = 1; i < rounds; i++) {
    await builder.addConsensusConnectedRound();
  }
  const randomness = new RoundRobin();
  const binder = new DAGRider({
    datastore: builder.datastore,
    randomness,
  });
  await binder.saveOrderedPageNumbers(1, rounds);
}

export default handler;

import cliCalls from "cli-calls";
await cliCalls(import.meta, handler);
