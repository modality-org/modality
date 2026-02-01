/**
 * modal hub grant - Grant access to a contract
 */

import { loadCredentials, createAuthHeaders } from "./lib.js";

export const command = "grant <contract> <identity> [permission]";
export const describe = "Grant access to a contract";

export function builder(yargs) {
  return yargs
    .positional("contract", {
      type: "string",
      describe: "Contract ID",
    })
    .positional("identity", {
      type: "string",
      describe: "Identity ID to grant access to",
    })
    .positional("permission", {
      type: "string",
      default: "read",
      choices: ["read", "write"],
      describe: "Permission level",
    })
    .option("creds", {
      alias: "c",
      type: "string",
      default: ".modal-hub/credentials.json",
      describe: "Path to credentials file",
    });
}

export async function handler(argv) {
  const { contract, identity, permission, creds: credsPath } = argv;
  
  const creds = loadCredentials(credsPath);
  if (!creds) return;
  
  try {
    const body = { identity_id: identity, permission };
    const path = `/contracts/${contract}/access`;
    const headers = await createAuthHeaders(creds, "POST", path, body);
    
    const res = await fetch(`${creds.hub_url}${path}`, {
      method: "POST",
      headers,
      body: JSON.stringify(body),
    });
    
    const data = await res.json();
    if (!res.ok) {
      console.error("❌", data.error);
      process.exit(1);
    }
    
    console.log(`✅ Granted ${permission} access`);
    console.log(`   Contract: ${contract}`);
    console.log(`   Identity: ${identity}`);
    
  } catch (err) {
    console.error("❌ Failed:", err.message);
    process.exit(1);
  }
}
