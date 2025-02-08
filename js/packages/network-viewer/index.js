import concurrently from "concurrently";
import { dirname } from "dirname-filename-esm";
const __dirname = dirname(import.meta);

export default async function main({ port = 3000, datastore }) {
  console.log(`STARTING on http://0.0.0.0:${port}`);
  concurrently(
    [
      `HOT_RELOAD=1 node ${__dirname}/api/index.mjs --port "${port + 1}" --datastore "${datastore}" --watch`,
      `node ${__dirname}/app/index.js --port "${port}"`,
    ],
    {
      killOthers: true,
    }
  );
}

import cliCalls from "cli-calls";
await cliCalls(import.meta, main);
