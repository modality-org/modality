import SafeJSON from "@modality-dev/utils/SafeJSON";

export const TOPIC = "/consensus/block/cert";

export async function handler(node, event) {
  const text = new TextDecoder().decode(event.detail.data);
  const obj = SafeJSON.parse(text);

  await node.services.local.consensus.onReceiveBlockCert(obj);
}
