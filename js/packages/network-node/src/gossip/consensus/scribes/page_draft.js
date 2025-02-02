import SafeJSON from "@modality-dev/utils/SafeJSON";
// import "@modality-dev/network-consensus";

export const TOPIC = "/consensus/scribes/page_draft";

export async function handler(node, event) {
  const text = new TextDecoder().decode(event.detail.data);
  const page = SafeJSON.parse(text);

  await node.services.local.consensus.onReceiveDraftPage(page);
}
