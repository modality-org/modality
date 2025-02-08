import { setupServer } from "@thylacine-js/webapi-express";
import NetworkDatastore from "@modality-dev/network-datastore";
import NetworkDatastoreBuilder from "@modality-dev/network-datastore/NetworkDatastoreBuilder";

import DAGRider from "@modality-dev/network-consensus/sequencing/DAGRider";
import RoundRobin from "@modality-dev/network-consensus/election/RoundRobin";
import chokidar from "chokidar";
import tmp from "tmp";
import path from "path";
import fs from "fs-extra";
import _ from "lodash";

import { dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const appDir = `${__dirname}/`;

export default async function main({ port, datastore, watch }) {
  port = port || 3001;

  const server = await setupServer({ appDir, validateCors: () => true });

  const dsTmpDir = tmp.dirSync({ prefix: "dsTmpDir" });
  let latestCopy = null;

  const copyAndLoadDatastoreInDirectory = async (datastore) => {
    const newCopy = Date.now().toString();
    fs.copySync(datastore, path.join(dsTmpDir.name, newCopy));
    server.datastore = await NetworkDatastore.createInDirectory(
      path.join(dsTmpDir.name, newCopy)
    );
    if (latestCopy) {
      fs.rmdirSync(path.join(dsTmpDir.name, latestCopy), {
        recursive: true,
        force: true,
      });
    }
    latestCopy = newCopy;
  };

  if (datastore === "mock") {
    const SCRIBES = 5;
    const ROUNDS = 12;
    const builder = await NetworkDatastoreBuilder.createInMemory();
    const scribes = await NetworkDatastoreBuilder.generateScribes(SCRIBES);
    builder.scribes = Object.keys(scribes);
    await builder.addFullyConnectedRound();
    for (let i = 1; i < ROUNDS; i++) {
      await builder.addConsensusConnectedRound();
    }
    const randomness = new RoundRobin();
    const binder = new DAGRider({
      datastore: builder.datastore,
      randomness,
    });
    await binder.saveOrderedBlockNumbers(1, ROUNDS);
    server.datastore_builder = builder;
    server.datastore = builder.datastore;
  } else if (datastore) {
    if (watch) {
      await copyAndLoadDatastoreInDirectory(datastore);
    } else {
      server.datastore = await NetworkDatastore.createInDirectory(datastore);
    }
  } else {
    server.datastore = await NetworkDatastore.createInMemory();
  }

  if (watch) {
    const debouncedCopyAndLoad = _.debounce(
      () => copyAndLoadDatastoreInDirectory(datastore),
      1000
    );
    chokidar.watch(datastore).on("all", (event, path) => {
      debouncedCopyAndLoad(datastore);
    });
  }

  server.listen(port, () => {
    console.log(`listening on http://0.0.0.0:${port}`);
  });
}

import cliCalls from "cli-calls";
cliCalls(import.meta, main);
