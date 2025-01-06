import fs from 'fs-extra';
import * as tar from 'tar';

import createLibp2pNode from "../createLibp2pNode.js";
import PeerIdHelpers from "../PeerIdHelpers.js";
import { parseConfigArgs } from "../parseConfigArgs.js";

import { addSequencerEventListeners } from "../gossip/index.js";
import { setupNode } from '../lib/setupNode.js';

async function addPeerDiscoveryEventListeners(node) {
  node.addEventListener("peer:connect", (evt) => {
    console.log("connected to: ", evt.detail.toString());
  });

  node.addEventListener("peer:discovery", (evt) => {
    console.log("found peer: ", evt.detail.toString());
  });
}

export default async function run({ config, keypair, listen, storage, load_storage, services }) {
  const conf = parseConfigArgs({ config, keypair, listen, storage });

  if (load_storage && conf.storage) {
    fs.ensureDirSync(conf.storage);
    fs.emptyDirSync(conf.storage);
    await tar.extract({
      file: load_storage,
      cwd: conf.storage,
    });
    console.log({load_storage, storage: conf.storage});
  }

  const peerId = await PeerIdHelpers.createFromJSON(conf.keypair);

  const node = await createLibp2pNode({
    peerId,
    addresses: {
      listen: [conf.listen],
    },
    bootstrappers: conf.bootstrappers,
  });

  await setupNode(node, conf);

  await addPeerDiscoveryEventListeners(node);
  services = Array.isArray(services) ? services : [services];
  if (services.includes("scribe") || services.includes("sequencer")) {
    await addSequencerEventListeners(node);
    console.log(`Starting on round: ${await node.storage.datastore.getCurrentRound()}`);
  }
 
  console.log("Listener ready, listening on:");
  node.getMultiaddrs().forEach((ma) => {
    console.log(ma.toString());
  });

  const abortController = new AbortController();
  const cleanup = async () => {
    abortController.abort();
  };

  process.on('SIGINT', async () => {
    console.log('Caught SIGINT (Ctrl+C)');
    await cleanup();
    process.exit(0);
  });
  
  process.on('SIGTERM', async () => {
    console.log('Caught SIGTERM');
    await cleanup();
    process.exit(0);
  });

  node.consensus.run(abortController.signal);
}

import cliCalls from "cli-calls";
await cliCalls(import.meta, run);
