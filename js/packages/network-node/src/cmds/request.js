import createLibp2pNode from "../createLibp2pNode.js";
import PeerIdHelpers from "../PeerIdHelpers.js";
import { multiaddr } from "@multiformats/multiaddr";
import { randomBytes } from "@noble/hashes/utils";
import { pipe } from "it-pipe";
import map from "it-map";
import * as lp from "it-length-prefixed";

import { streamToConsole } from "../StreamHelpers.js";
import { parseConfigArgs } from "../parseConfigArgs.js";

export default async function main({
  config,
  keypair,
  listen,
  storage,
  target,
  path,
  data,
}) {
  const conf = parseConfigArgs({ config, keypair, listen, storage });
  const peerId = await PeerIdHelpers.createFromJSON(conf.keypair);
  const node = await createLibp2pNode({
    peerId,
  });

  const ma = multiaddr(target);

  const res = await node.services.reqres.call(ma, path, data);
  console.log(res);

  await node.stop();
}

import cliCalls from "cli-calls";
await cliCalls(import.meta, main);
