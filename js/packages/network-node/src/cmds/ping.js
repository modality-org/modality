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
  times = 1,
}) {
  const conf = parseConfigArgs({ config, keypair, listen, storage });
  const peerId = await PeerIdHelpers.createFromJSON(conf.keypair);
  const node = await createLibp2pNode({
    peerId,
  });

  const ma = multiaddr(target);
  const stream = await node.dialProtocol(ma, "/ipfs/ping/1.0.0");

  const start = Date.now();
  for (const i of Array.from({ length: times })) {
    const data = randomBytes(32);
    console.time("pinged in");
    await pipe([data], stream, async (stream) => {
      for await (const chunk of stream) {
        const v = chunk.subarray();
        const byteMatches = v.every((byte, i) => byte === data[i]);
        if (!byteMatches) {
          throw new Error("Wrong pong");
        }
      }
    });
    console.timeEnd("pinged in");
  }
  await stream.close();
  console.log(
    "pinged",
    times,
    "times in",
    Date.now() - start,
    "ms",
    "(",
    (times / (Date.now() - start)) * 1000,
    "pings/s )"
  );

  await node.stop();
}

import cliCalls from "cli-calls";
await cliCalls(import.meta, main);
