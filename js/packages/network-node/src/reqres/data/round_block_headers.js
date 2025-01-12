export const PATH = "/data/round_block_headers";

import RoundBlockHeader from '@modality-dev/network-datastore/data/RoundBlockHeader';

export async function handler({ datastore, peer, data }) {
  const round_id = data.round_id;
  if (!round_id) {
    return { ok: false, error: 'missing round_id' };
  }
  const round_block_headers = await RoundBlockHeader.findAllInRound({ datastore, round_id });
  return {
    ok: true,
    data: {
      round_block_headers
    }
  };
}
