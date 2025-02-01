import NetworkDatastore from "@modality-dev/network-datastore";

class ModalityStorageService {
  static serviceName = 'storage'
  
  static dependencies = {
  }

  constructor(config = {}) {
    this.config = config;
  }

  async init({ datastore }) { 
  }

  async start() {
    if (this.config?.storage_path) {
      this.datastore = await NetworkDatastore.createWith({
        storage_type: "directory",
        storage_path: this.config.storage_path,
      });
    } else {
      this.datastore = await NetworkDatastore.createInMemory();
    }
    if (this.config.network_config) {
      await this.datastore.loadNetworkConfig(this.config.network_config);
    }
    // console.log('Service starting...')
  }

  async stop() {
    // console.log('Service stopping...')
  }
}

export default function (config) {
  return (components, options) => new ModalityStorageService(config);
}
