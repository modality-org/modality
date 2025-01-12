import NetworkDatastore from "@modality-dev/network-datastore";

export async function setupStorage(node, conf) {
  node.storage ||= {};
  // node.services.storage ||= {};
  let ds;
  if (conf.storage) {
    ds = await NetworkDatastore.createWith({
      storage_type: "directory",
      storage_path: conf.storage,
    });
  } else {
    ds = await NetworkDatastore.createInMemory();
  }
  node.storage.datastore = ds;
  // node.services.storage.datastore = ds;
}
