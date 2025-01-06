import { setupStorage } from "./setupStorage";
import { setupConsensus } from "./setupConsensus";

export async function setupNode(node, conf) {
  await setupStorage(node, conf);
  await setupConsensus(node, conf);
}
