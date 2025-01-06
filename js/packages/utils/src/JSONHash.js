import { createHash } from "crypto";
import JSONStringifyDeterministic from "json-stringify-deterministic";

export default function JSONHash(json) {
  if (!json) {
    return null;
  }
  const str = JSONStringifyDeterministic(json);
  return createHash("sha256").update(str).digest("hex");
}
