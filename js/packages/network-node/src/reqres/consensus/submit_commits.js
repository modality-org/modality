export const PATH = "/consensus/submit_commits";

export function handler({ peer, data }) {
  console.log({ data });
  return { ok: true };
}
