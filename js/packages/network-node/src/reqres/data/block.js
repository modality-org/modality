export const PATH = "/data/block";

import Block from '@modality-dev/network-datastore/data/Block';

export async function handler({ datastore, peer, data }) {
  const round_id = data.round_id;
  const peer_id = data.peer_id;
  if (round_id == null) {
    return { ok: false, error: 'missing round_id' };
  }
  if (!peer_id) {
    return { ok: false, error: 'missing peer_id' };
  }
  const block = await Block.findOne({datastore, round_id, peer_id});
  return {
    ok: true,
    data: {
      block: block?.toJSON()
    }
  };
}
