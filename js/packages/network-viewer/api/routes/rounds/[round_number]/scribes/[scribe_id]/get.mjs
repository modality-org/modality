import Block from "@modality-dev/network-datastore/data/Block";
import Round from "@modality-dev/network-datastore/data/Round";
import { setupSequencing } from "@modality-dev/network-consensus";

export default async function (req, res) {
  const round_number = parseInt(req.params.round_number);
  const scribe_id = req.params.scribe_id;

  const datastore = req.app.datastore;

  let prev_round;
  try {
    prev_round = await Round.findOne({ round: round_number - 1, datastore });
  } catch (e) {
    //
  }

  const sequencing = setupSequencing({
    datastore,
    sequencing_method: "DAGRider",
    election_method: "RoundRobin",
  });

  const prev_round_scribes_count = prev_round?.scribes.length;
  const prev_round_threshold = sequencing.consensusThresholdForRound(
    round_number - 1
  );
  const leader = await sequencing.findLeaderInRound(round_number);
  const leader_scribe = leader?.scribe;
  const is_section_leader = leader_scribe === scribe_id;

  const page = await Block.findOne({
    round: round_number,
    scribe: scribe_id,
    datastore,
  });
  const is_certified = Object.keys(page.acks).length >= prev_round_threshold;

  return res.json({
    ok: true,
    data: {
      page: {
        ...page,
        is_certified,
        is_section_leader,
      },
    },
  });
}
