export const PATH = "/status";

export async function handler({ datastore, peer, data }) {
  const status = await datastore.getStatus();

  return {
    ok: true,
    data: {
      ...status,
    },
  };
}
