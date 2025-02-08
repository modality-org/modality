export const PATH = "/data/round/block_headers";

import BlockHeader from "@modality-dev/network-datastore/data/BlockHeader";

export async function handler({ datastore, peer, data }) {
  const round_id = data.round_id;
  if (round_id == null) {
    return { ok: false, error: "missing round_id" };
  }
  await BlockHeader.derviveAllInRound({ datastore, round_id });
  const round_block_headers_records = await BlockHeader.findAllInRound({
    datastore,
    round_id,
  });
  const round_block_headers = round_block_headers_records.map((i) =>
    i.toJSONObject()
  );
  return {
    ok: true,
    data: {
      round_block_headers,
    },
  };
}
