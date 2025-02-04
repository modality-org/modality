export const PATH = "/consensus/block/ack";

export async function handler({ peer, data, local }) {
  await local.consensus?.onReceiveBlockAck(data);

  return { ok: true };
}
