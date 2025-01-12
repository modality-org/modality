import { createLibp2p } from "libp2p";

// protocols
import { tcp } from "@libp2p/tcp";
import { webSockets } from "@libp2p/websockets";
import { ping } from "@libp2p/ping";

// encryption
import { noise } from "@chainsafe/libp2p-noise";
import { plaintext } from "@libp2p/plaintext";

// multiplexers
import { yamux } from "@chainsafe/libp2p-yamux";

// peer discovery
import { bootstrap } from "@libp2p/bootstrap";

// services
import { identify } from "@libp2p/identify";
import { gossipsub } from "@chainsafe/libp2p-gossipsub";
import reqres from "./reqres/index.js";
import storage from './storage/index.js';

export default async function createLibp2pNode({
  port,
  enableNAT,
  disableEncryption,
  disableBootstrap,
  enableServeAsRelay,
  enableListenViaRelay,
  bootstrappers,
  peerId,
  privateKey,
  ...options
} = {}) {
  const transports = [tcp(), webSockets()];

  const connectionEncrypters = disableEncryption ? [plaintext()] : [noise()];

  const nat = enableNAT
    ? {
      enabled: true,
    }
    : {};

  const relay = {
    enabled: true,
  };
  if (enableServeAsRelay) {
    relay.hop = {
      enabled: true,
    };
    relay.advertise = {
      enabled: true,
    };
  }
  if (enableListenViaRelay) {
    relay.autoRelay = {
      enabled: true,
      maxListeners: 2,
    };
  }

  bootstrappers = bootstrappers?.filter(
    (i) => !i.match(`p2p/${peerId.toString()}$`)
  );

  const node = await createLibp2p({
    transports,
    connectionEncrypters,
    streamMuxers: [yamux()],
    relay,
    nat,
    peerDiscovery: [
      ...(disableBootstrap || !bootstrappers?.length
        ? []
        : [bootstrap({ list: bootstrappers })]),
    ],
    services: {
      storage: storage({storage_path: options.storage_path}),
      identify: identify(),
      ping: ping(),
      pubsub: gossipsub({
        emitSelf: true
      }),
      reqres: reqres(),
    },
    start: false,
    peerId,
    privateKey,
    ...options,
  });

  const stop = async () => {
    await node.stop();
    process.exit(0);
  };

  process.on("SIGTERM", stop);
  process.on("SIGINT", stop);

  return node;
}
