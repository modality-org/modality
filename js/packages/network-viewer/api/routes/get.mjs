import Round from '@modality-dev/network-datastore/data/Round';

export default async function (req, res) {
  const datastore = req.app.datastore;
  const round = await Round.findMaxId({datastore})

  return res.json({
    ok: true, data: {
      status: {
        round
      }
    }
  });
}
