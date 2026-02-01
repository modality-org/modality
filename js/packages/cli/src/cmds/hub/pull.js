/**
 * modal hub pull - Pull commits from a contract
 */

import { existsSync, writeFileSync, mkdirSync } from "fs";
import { dirname, join } from "path";
import { loadCredentials, createAuthHeaders } from "./lib.js";

export const command = "pull <contract>";
export const describe = "Pull commits from a contract on the hub";

export function builder(yargs) {
  return yargs
    .positional("contract", {
      type: "string",
      describe: "Contract ID",
    })
    .option("since", {
      alias: "s",
      type: "string",
      describe: "Pull commits after this hash",
    })
    .option("output", {
      alias: "o",
      type: "string",
      describe: "Output directory (extracts files)",
    })
    .option("json", {
      type: "boolean",
      default: false,
      describe: "Output raw JSON",
    })
    .option("creds", {
      alias: "c",
      type: "string",
      default: ".modal-hub/credentials.json",
      describe: "Path to credentials file",
    });
}

export async function handler(argv) {
  const { contract, since, output, json, creds: credsPath } = argv;
  
  const creds = loadCredentials(credsPath);
  if (!creds) return;
  
  try {
    let path = `/contracts/${contract}/pull`;
    if (since) path += `?since=${since}`;
    
    const headers = await createAuthHeaders(creds, "GET", path);
    
    const res = await fetch(`${creds.hub_url}${path}`, { headers });
    const data = await res.json();
    
    if (!res.ok) {
      console.error("❌", data.error);
      process.exit(1);
    }
    
    if (json) {
      console.log(JSON.stringify(data, null, 2));
      return;
    }
    
    console.log(`📥 Contract: ${contract}`);
    console.log(`   Head: ${data.head || "(empty)"}`);
    console.log(`   Commits: ${data.commits.length}`);
    console.log("");
    
    for (const commit of data.commits) {
      const method = commit.data?.method || "DATA";
      const commitPath = commit.data?.path || "";
      console.log(`   ${commit.hash.slice(0, 8)} ${method} ${commitPath}`);
    }
    
    // Extract files if output directory specified
    if (output) {
      if (!existsSync(output)) {
        mkdirSync(output, { recursive: true });
      }
      
      let extracted = 0;
      for (const commit of data.commits) {
        if (commit.data?.content && commit.data?.path) {
          const filePath = join(output, commit.data.path);
          const dir = dirname(filePath);
          
          if (!existsSync(dir)) {
            mkdirSync(dir, { recursive: true });
          }
          
          writeFileSync(filePath, commit.data.content);
          extracted++;
        }
      }
      
      if (extracted > 0) {
        console.log(`\n✅ Extracted ${extracted} file(s) to ${output}`);
      }
    }
    
  } catch (err) {
    console.error("❌ Failed:", err.message);
    process.exit(1);
  }
}
