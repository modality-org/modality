import JSONFile from "@modality-dev/utils/JSONFile";
import Keypair from "@modality-dev/utils/Keypair";
import {
  resolveDnsEntries,
  matchesPeerIdSuffix,
} from "@modality-dev/utils/MultiaddrList";
import path from "path";
import setupNodeInternals from "./setupNodeInternals.js";
import { addSequencerEventListeners } from "./gossip/index.js";
import { setupNetworkConsensus } from "@modality-dev/network-consensus";
import ConsensusCommunication from "../src/lib/ConsensusCommunication.js";
import { peerIdFromString } from "@libp2p/peer-id";
export default class Node {
  constructor({
    peerid,
    keypair,
    listeners,
    bootstrappers,
    swarm,
    storage_path,
    network_config,
  }) {
    this.peerid = peerid;
    this.keypair = keypair;
    this.listeners = listeners;
    this.bootstrappers = bootstrappers;
    this.storage_path = storage_path;
    this.network_config = network_config;
    this.swarm = swarm;
  }

  static async fromConfigFilepath(filepath, overrides = {}) {
    const config = JSONFile.readSync(filepath);
    const relative_path_base = path.resolve(path.dirname(filepath));
    config.passfile_path = path.resolve(
      relative_path_base,
      config.passfile_path
    );
    config.storage_path = path.resolve(relative_path_base, config.storage_path);
    config.network_config_path =
      config.network_config_path &&
      path.resolve(relative_path_base, config.network_config_path);
    return Node.fromConfig({ ...config, ...overrides });
  }

  static async fromConfig(config) {
    const keypair = await Keypair.fromJSONFile(config.passfile_path);
    const peerid = await keypair.asPublicAddress();
    const storage_path = config.storage_path;
    const listeners = config.listeners || [];
    const resolved_bootstrappers = await resolveDnsEntries(
      config.bootstrappers || []
    );
    const bootstrappers = resolved_bootstrappers.filter(
      (ma) => !matchesPeerIdSuffix(ma, peerid)
    );
    let network_config = null;
    if (config.network_config_path) {
      network_config = JSONFile.readSync(config.network_config_path);
    }
    const node = new Node({
      peerid,
      keypair,
      storage_path,
      listeners,
      bootstrappers,
      network_config,
    });

    return node;
  }

  static async createNetworkClient(network = "mainnet", storage_path = null) {
    const keypair = await Keypair.generate();
    const peerid = await keypair.asPublicAddress();
    const resolved_bootstrappers = await resolveDnsEntries([
      `/dnsaddr/${network}.modality.network`,
    ]);
    const bootstrappers = resolved_bootstrappers.filter(
      (ma) => !matchesPeerIdSuffix(ma, peerid)
    );
    const listeners = [];
    const node = new Node({
      peerid,
      keypair,
      listeners,
      bootstrappers,
      storage_path,
    });
    return node;
  }

  async setup(mode = "client") {
    const peerId = await this.keypair.asPeerId(); //await PeerIdHelpers.createFromJSON(await this.keypair.asJSON());
    const privateKey = this.keypair.key;
    const addresses = mode === "client" ? {} : { listen: this.listeners };
    const swarm = await setupNodeInternals({
      peerId,
      privateKey,
      addresses,
      bootstrappers: this.bootstrappers,
      storage_path: this.storage_path,
      network_config: this.network_config,
    });
    this.swarm = swarm;
    await this.swarm.start();
  }

  async setupAsClient() {
    return this.setup("client");
  }

  async setupAsServer() {
    return this.setup("server");
  }

  async listenForConsensusEvents() {
    addSequencerEventListeners(this.swarm);
  }

  async stop() {
    await this.swarm.stop();
  }

  async reqres() {
    return this.swarm(...arguments);
  }

  async getPeerId() {
    return this.keypair.asPeerId();
  }

  getListenerMultiaddress() {
    return this.swarm.getMultiaddrs()?.[0];
  }

  async addPeerMultiaddress(peer_id, multiaddress) {
    return this.swarm.peerStore.save(peer_id, { multiaddrs: [multiaddress] });
  }

  getDatastore() {
    return this.swarm?.services?.local?.datastore;
  }

  sendRequest(to, path, data) {
    return this.swarm.services.reqres.call(
      typeof to === "string" ? peerIdFromString(to) : to,
      path,
      data
    );
  }

  handleRequest(from, path, data) {
    const context = {
      services: this.swarm.services,
      datastore: this.swarm.services.local.datastore,
      local: this.swarm.services.local,
    };
    return this.swarm.services.reqres.handleRequest(from, path, data, context);
  }

  sendOrHandleRequest(to, path, data) {
    if (to === this.peerid) {
      return this.handleRequest(this.peerid, path, data);
    } else {
      return this.sendRequest(to, path, data);
    }
  }

  publishGossip(topic, data) {
    const json_text = new TextEncoder().encode(JSON.stringify(data));
    return this.swarm.services.pubsub.publish(topic, json_text);
  }

  async setupLocalConsensus(opts = {}) {
    const consensus = await setupNetworkConsensus({
      peerid: this.peerid,
      keypair: this.keypair,
      datastore: this.getDatastore(),
      sequencing_method: "StaticAuthority",
      election_method: "RoundRobin",
      ...opts,
    });
    consensus.communication = new ConsensusCommunication({
      node: this,
    });
    await this.listenForConsensusEvents();
    this.swarm.services.local.consensus = consensus;
    return consensus;
  }

  async waitForConnections() {
    console.log("connecting to network...")
    return new Promise(r => {
      let interval = setInterval(async () => {
        console.log("connecting to network...")
        const connections = this.swarm.getConnections();
        if (connections.length > 0) {
          clearInterval(interval);
          r();
        }
      }, 15*1000);
    });
  }
}
