export const PATH = "/data/round_block_headers";

import RoundBlockHeader from '@modality-dev/network-datastore/data/RoundBlockHeader';

export async function handler({ datastore, peer, data }) {
  const round_id = data.round_id;
  if (!round_id) {
    return { ok: false, error: 'missing round_id' };
  }
  const round_block_headers_records = await RoundBlockHeader.findAllInRound({ datastore, round_id });
  const round_block_headers = round_block_headers_records.map(i => i.toJSONObject());
  return {
    ok: true,
    data: {
      round_block_headers
    }
  };
}
