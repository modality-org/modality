export const PATH = "/ping";

export function handler({ peer, data }) {
  return {
    ok: true,
    data
   };
}
