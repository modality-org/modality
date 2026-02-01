/**
 * modal hub push - Push commits to a contract
 */

import { existsSync, readFileSync } from "fs";
import { createHash } from "crypto";
import { loadCredentials, createAuthHeaders } from "./lib.js";

export const command = "push <contract>";
export const describe = "Push commits to a contract on the hub";

export function builder(yargs) {
  return yargs
    .positional("contract", {
      type: "string",
      describe: "Contract ID",
    })
    .option("file", {
      alias: "f",
      type: "string",
      describe: "File to push (creates a POST commit)",
    })
    .option("path", {
      alias: "p",
      type: "string",
      describe: "Path in contract (e.g., /state/data.json)",
    })
    .option("rule", {
      alias: "r",
      type: "string",
      describe: "Rule file to push (creates a RULE commit)",
    })
    .option("message", {
      alias: "m",
      type: "string",
      describe: "Commit message",
    })
    .option("creds", {
      alias: "c",
      type: "string",
      default: ".modal-hub/credentials.json",
      describe: "Path to credentials file",
    });
}

export async function handler(argv) {
  const { contract, file, path: commitPath, rule, message, creds: credsPath } = argv;
  
  const creds = loadCredentials(credsPath);
  if (!creds) return;
  
  // Build commit
  let commits = [];
  
  // First, get current head
  let parentHash = null;
  try {
    const headers = await createAuthHeaders(creds, "GET", `/contracts/${contract}`);
    const res = await fetch(`${creds.hub_url}/contracts/${contract}`, { headers });
    const data = await res.json();
    if (res.ok && data.head) {
      parentHash = data.head;
    }
  } catch {
    // No head yet, that's fine
  }
  
  if (file) {
    if (!existsSync(file)) {
      console.error(`❌ File not found: ${file}`);
      process.exit(1);
    }
    
    const content = readFileSync(file, "utf8");
    const filePath = commitPath || `/${file}`;
    
    const commitData = {
      method: "POST",
      path: filePath,
      content,
      message,
    };
    
    commits.push({
      hash: hashCommit(commitData, parentHash),
      parent: parentHash,
      data: commitData,
    });
  }
  
  if (rule) {
    if (!existsSync(rule)) {
      console.error(`❌ Rule file not found: ${rule}`);
      process.exit(1);
    }
    
    const content = readFileSync(rule, "utf8");
    const rulePath = commitPath || `/rules/${rule}`;
    
    const commitData = {
      method: "RULE",
      path: rulePath,
      content,
      message,
    };
    
    // If we already have a commit, chain it
    const parent = commits.length > 0 ? commits[commits.length - 1].hash : parentHash;
    
    commits.push({
      hash: hashCommit(commitData, parent),
      parent,
      data: commitData,
    });
  }
  
  if (commits.length === 0) {
    console.error("❌ Nothing to push. Use --file or --rule");
    process.exit(1);
  }
  
  // Push commits
  try {
    const body = { commits };
    const headers = await createAuthHeaders(creds, "POST", `/contracts/${contract}/push`, body);
    
    const res = await fetch(`${creds.hub_url}/contracts/${contract}/push`, {
      method: "POST",
      headers,
      body: JSON.stringify(body),
    });
    
    const data = await res.json();
    if (!res.ok) {
      console.error("❌", data.error);
      process.exit(1);
    }
    
    console.log(`✅ Pushed ${data.pushed} commit(s)`);
    console.log(`   Head: ${data.head}`);
    
    for (const c of commits) {
      console.log(`   ${c.hash.slice(0, 8)} ${c.data.method} ${c.data.path}`);
    }
    
  } catch (err) {
    console.error("❌ Failed:", err.message);
    process.exit(1);
  }
}

function hashCommit(data, parent) {
  const payload = JSON.stringify({ data, parent });
  return createHash("sha256").update(payload).digest("hex").slice(0, 16);
}
