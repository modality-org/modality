/**
 * modal hub create - Create a new contract on the hub
 */

import { existsSync, readFileSync } from "fs";
import { loadCredentials, createAuthHeaders } from "./lib.js";

export const command = "create [name]";
export const describe = "Create a new contract on the hub";

export function builder(yargs) {
  return yargs
    .positional("name", {
      type: "string",
      describe: "Contract name",
    })
    .option("description", {
      alias: "d",
      type: "string",
      describe: "Contract description",
    })
    .option("creds", {
      alias: "c",
      type: "string",
      default: ".modal-hub/credentials.json",
      describe: "Path to credentials file",
    });
}

export async function handler(argv) {
  const { name, description, creds: credsPath } = argv;
  
  const creds = loadCredentials(credsPath);
  if (!creds) return;
  
  try {
    const headers = await createAuthHeaders(creds, "POST", "/contracts", { name, description });
    
    const res = await fetch(`${creds.hub_url}/contracts`, {
      method: "POST",
      headers,
      body: JSON.stringify({ name, description }),
    });
    
    const data = await res.json();
    if (!res.ok) {
      console.error("❌", data.error);
      process.exit(1);
    }
    
    console.log("✅ Contract created");
    console.log(`   ID: ${data.contract_id}`);
    console.log(`   Owner: ${data.owner}`);
    
    if (name) console.log(`   Name: ${name}`);
    
  } catch (err) {
    console.error("❌ Failed:", err.message);
    process.exit(1);
  }
}
