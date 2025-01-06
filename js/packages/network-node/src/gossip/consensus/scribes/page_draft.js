import SafeJSON from "@modality-dev/utils/SafeJSON";
// import "@modality-dev/network-consensus";

export const TOPIC = "/consensus/scribes/page_draft";

export async function handler(node, event) {
  const text = new TextDecoder().decode(event.detail.data);
  const page = SafeJSON.parse(text);

  await node.consensus.onReceiveDraftPage(page);
  // const page_ack = { };
  // await node.services.reqres.call(
  //   page.scribe,
  //   "/consensus/scribes/page_ack",
  //   page_ack
  // );
  // return page_ack;
}
