export const PATH = "/consensus/scribes/page_ack";

export async function handler({ peer, data, node }) {
  // const text = new TextDecoder().decode(event.detail.data);
  // const page = SafeJSON.parse(text);

  await node.services.local.consensus?.onReceivePageAck(data);

  // TODO record signed vertex (timely or late)
  // if threshold met for signed vertices:
  //   * record vertex certificate
  //   * gossip vertex certificate
  return { ok: true };
}
