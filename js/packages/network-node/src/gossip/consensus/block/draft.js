import SafeJSON from "@modality-dev/utils/SafeJSON";
// import "@modality-dev/network-consensus";

export const TOPIC = "/consensus/block/draft";

export async function handler(node, event) {
  const text = new TextDecoder().decode(event.detail.data);
  const block_data = SafeJSON.parse(text);

  await node.services.local.consensus.onReceiveBlockDraft(block_data);
}
