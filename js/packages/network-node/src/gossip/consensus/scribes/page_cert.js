import SafeJSON from "@modality-dev/utils/SafeJSON";

export const TOPIC = "/consensus/scribes/page_cert";

export async function handler(node, event) {
  const text = new TextDecoder().decode(event.detail.data);
  const obj = SafeJSON.parse(text);

  await node.services.local.consensus.onReceiveCertifiedPage(obj);
}
