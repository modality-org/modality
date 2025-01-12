import JSONFile from "@modality-dev/utils/JSONFile";
import Keypair from "@modality-dev/utils/Keypair"
import { resolveDnsEntries, matchesPeerIdSuffix } from "@modality-dev/utils/MultiaddrList";
import path from 'path';
import createLibp2pNode from "./createLibp2pNode.js";
import PeerIdHelpers from "./PeerIdHelpers.js";
import NetworkDatastore from "@modality-dev/network-datastore";
export default class Node {
  constructor({ peerid, keypair, listeners, bootstrappers, swarm }) {
    this.peerid = peerid;
    this.keypair = keypair;
    this.listeners = listeners;
    this.bootstrappers = bootstrappers;
    this.swarm = swarm;
  }

  static async fromConfigFilepath(filepath) {
    const config = JSONFile.readSync(filepath);
    const relative_path_base = path.resolve(path.dirname(filepath));
    config.passfile_path = path.resolve(relative_path_base, config.passfile_path);
    config.storage_path = path.resolve(relative_path_base, config.storage_path);
    return Node.fromConfig(config);
  }

  static async fromConfig(config) {
    const keypair = await Keypair.fromJSONFile(config.passfile_path);
    const peerid = await keypair.asPublicAddress();
    const storage_path = config.storage_path;
    const listeners = config.listeners || [];
    const resolved_bootstrappers = await resolveDnsEntries(config.bootstrappers || []);
    const bootstrappers = resolved_bootstrappers.filter(ma => !matchesPeerIdSuffix(ma, peerid));

    const node = new Node({ peerid, keypair, storage_path, listeners, bootstrappers });

    return node;
  }

  async setup(mode = 'client') {
    const peerId = await this.keypair.asPeerId(); //await PeerIdHelpers.createFromJSON(await this.keypair.asJSON());
    const privateKey = this.keypair.key;
    const addresses = mode === 'client' ? {} : { listen: this.listeners };
    let ds;
    if (this.storage_path) {
      ds = await NetworkDatastore.createWith({
        storage_type: "directory",
        storage_path: this.storage_path,
      });
    } else {
      ds = await NetworkDatastore.createInMemory();
    }
    const swarm = await createLibp2pNode({
      peerId,
      privateKey,
      addresses,
      bootstrappers: this.bootstrappers,
    });
    this.swarm = swarm;
    await this.swarm.start();
  }

  async setupAsClient() {
    return this.setup('client');
  }

  async setupAsServer() {
    return this.setup('server');
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
    return this.swarm.peerStore.save(
      peer_id,
      { multiaddrs: [multiaddress] }
    );
  }

  getDatastore() {
    return this.swarm?.services?.storage?.datastore;
  }
}