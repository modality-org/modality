import Round from "@modality-dev/network-datastore/data/Round";

export default async function (req, res) {
  const round_number = parseInt(req.params.round_number);

  const datastore = req.app.datastore;
  const round = await Round.findOne({ round: round_number, datastore });

  return res.json({
    ok: true,
    data: {
      round: {
        ...round,
        sequencing_method: "DAG Rider",
        sequencing_options: "",
      },
    },
  });
}
