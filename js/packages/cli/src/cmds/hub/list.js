/**
 * modal hub list - List your contracts
 */

import { loadCredentials, createAuthHeaders } from "./lib.js";

export const command = "list";
export const describe = "List your contracts on the hub";

export function builder(yargs) {
  return yargs
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
  const { json, creds: credsPath } = argv;
  
  const creds = loadCredentials(credsPath);
  if (!creds) return;
  
  try {
    const headers = await createAuthHeaders(creds, "GET", "/contracts");
    
    const res = await fetch(`${creds.hub_url}/contracts`, { headers });
    const data = await res.json();
    
    if (!res.ok) {
      console.error("❌", data.error);
      process.exit(1);
    }
    
    if (json) {
      console.log(JSON.stringify(data, null, 2));
      return;
    }
    
    if (data.contracts.length === 0) {
      console.log("📭 No contracts yet");
      console.log("   Create one with: modal hub create <name>");
      return;
    }
    
    console.log(`📋 Your contracts (${data.contracts.length}):\n`);
    
    for (const c of data.contracts) {
      console.log(`   ${c.id}`);
      if (c.name) console.log(`      Name: ${c.name}`);
      console.log(`      Head: ${c.head || "(empty)"}`);
      console.log(`      Updated: ${new Date(c.updated_at).toISOString()}`);
      console.log("");
    }
    
  } catch (err) {
    console.error("❌ Failed:", err.message);
    process.exit(1);
  }
}
