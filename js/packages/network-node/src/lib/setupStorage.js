import NetworkDatastore from "@modality-dev/network-datastore";

export async function setupStorage(node, conf) {
  node.storage ||= {};
  if (conf.storage) {
    node.storage.datastore = await NetworkDatastore.createWith({
      storage_type: "directory",
      storage_path: conf.storage,
    });
  } else {
    node.storage.datastore = await NetworkDatastore.createInMemory()
  }
}
