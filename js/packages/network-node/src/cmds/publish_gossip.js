import createLibp2pNode from "../createLibp2pNode.js";
import PeerIdHelpers from "../PeerIdHelpers.js";
import { multiaddr } from "@multiformats/multiaddr";
import { randomBytes } from "@noble/hashes/utils";
import { pipe } from "it-pipe";
import map from "it-map";
import * as lp from "it-length-prefixed";
import delay from "delay";
import pWaitFor from "p-wait-for";

import { streamToConsole } from "../StreamHelpers.js";
import { parseConfigArgs } from "../parseConfigArgs.js";

export default async function main({
  config,
  keypair,
  listen,
  storage,
  topic,
  message,
}) {
  const conf = parseConfigArgs({ config, keypair, listen, storage });
  const peerId = await PeerIdHelpers.createFromJSON(conf.keypair);
  const node = await createLibp2pNode({
    peerId,
    bootstrappers: conf.bootstrappers,
  });

  await pWaitFor(() => {
    return !!node.services.pubsub.getPeers().length;
  });
  await delay(1000);
  await node.services.pubsub.publish(topic, new TextEncoder().encode(message));

  await node.stop();
}

import cliCalls from "cli-calls";
await cliCalls(import.meta, main);
