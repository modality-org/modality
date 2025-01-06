import { readFile } from "fs/promises";

const genesis = JSON.parse(
  await readFile(new URL("./genesis.json", import.meta.url))
);
const keys = JSON.parse(
  await readFile(new URL("./keys.json", import.meta.url))
);
const bootstrappers = JSON.parse(
  await readFile(new URL("./bootstrappers.json", import.meta.url))
);

export default {
  name: "devnet-static1",
  genesis,
  keys,
  bootstrappers,
};
