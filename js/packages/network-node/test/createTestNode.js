import PeerIdHelpers from "../src/PeerIdHelpers.js";
import { ReqResService as SourceReqResService } from "../src/reqres/index.js";

class TestPeers {
  static peers_by_peerId = new Map();
  static peers_by_multiaddr = new Map();

  static addPeer(peerId, multiaddr, peer) {
    this.peers_by_peerId.set(peerId, peer);
    this.peers_by_multiaddr.set(multiaddr, peer);
  }

  static byPeerId(peerId) {
    return this.peers_by_peerId.get(peerId);
  }

  static byMultiaddr(multiaddr) {
    return this.peers_by_multiaddr.get(multiaddr);
  }

  static forEach(fn) {
    return this.peers_by_peerId.forEach(fn);
  }
}

class GossipSub extends EventTarget {
  static topics_to_peers_set = new Map();

  constructor(peers, multiaddr, peerId) {
    super();
    this.peers = peers;
    this.multiaddr = multiaddr;
    this.peerId = peerId;
  }

  async publish(topic, data) {
    const peer_set =
      this.constructor.topics_to_peers_set.get(topic) || new Set();
    for (const peerId of peer_set) {
      if (this.peerId === peerId) {
        continue;
      }
      const peer = this.peers.byPeerId(peerId);
      await peer.services.pubsub.doDispatchEvent({ topic, data });
    }
  }

  async subscribe(topic) {
    const peer_set =
      this.constructor.topics_to_peers_set.get(topic) || new Set();
    peer_set.add(this.peerId.toString());
    this.constructor.topics_to_peers_set.set(topic, peer_set);
  }

  doDispatchEvent({ topic, data }) {
    this.dispatchEvent(new CustomEvent("message", { detail: { topic, data } }));
  }
}

function gossipsub(peers, multiaddr, peerId) {
  return new GossipSub(peers, multiaddr, peerId);
}

class ReqResService {
  constructor(peers, multiaddr, peerId) {
    this.peers = peers;
    this.multiaddr = multiaddr;
    this.peerId = peerId;
  }

  async handleRequest(peer, path, data, options) {
    const r = await SourceReqResService.handleRequest(
      peer,
      path,
      data,
      options
    );
    return r;
  }

  async call(peerId, path, data, options) {
    const target_node = this.peers.peers_by_peerId.get(peerId.toString());
    if (!target_node) {
      throw new Error(`peer not found`);
    }
    const r = await target_node.services.reqres.handleRequest(
      peerId,
      path,
      data,
      options
    );
    return r;
  }
}

function reqres(peers, multiaddr, peerId) {
  return new ReqResService(peers, multiaddr, peerId);
}

export default async function createTestNode({
  keypair,
  listen,
  ...options
} = {}) {
  const peerId = await PeerIdHelpers.createFromJSON(keypair);
  const multiaddr = listen;

  // todo add test node to global list

  const node = {
    services: {
      pubsub: gossipsub(TestPeers, multiaddr, peerId),
      reqres: reqres(TestPeers, multiaddr, peerId),
    },
    peerId,
    addresses: { listen: [multiaddr] },
  };

  TestPeers.addPeer(peerId.toString(), multiaddr, node);

  return node;
}
