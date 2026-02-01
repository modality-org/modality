/**
 * modal hub register - Register identity and create access key
 */

import { existsSync, writeFileSync, readFileSync, mkdirSync } from "fs";
import { join, dirname } from "path";
import * as ed from "@noble/ed25519";
import { sha512 } from "@noble/hashes/sha512";

// Configure ed25519
ed.etc.sha512Sync = (...m) => sha512(ed.etc.concatBytes(...m));

export const command = "register";
export const describe = "Register identity and create access key";

export function builder(yargs) {
  return yargs
    .option("url", {
      alias: "u",
      type: "string",
      default: "http://localhost:3100",
      describe: "Hub URL",
    })
    .option("output", {
      alias: "o",
      type: "string",
      default: "./.modal-hub/credentials.json",
      describe: "Output file for credentials",
    })
    .option("name", {
      type: "string",
      describe: "Name for the access key",
    });
}

export async function handler(argv) {
  const { url, output, name } = argv;
  
  console.log("🔐 Registering with Contract Hub...\n");
  
  // Check if credentials already exist
  if (existsSync(output)) {
    console.log(`⚠️  Credentials file already exists: ${output}`);
    console.log("   Delete it first if you want to re-register");
    return;
  }
  
  // Generate identity keypair
  console.log("1️⃣  Generating identity keypair...");
  const identityPrivate = ed.utils.randomPrivateKey();
  const identityPublic = await ed.getPublicKeyAsync(identityPrivate);
  const identityPrivateHex = bytesToHex(identityPrivate);
  const identityPublicHex = bytesToHex(identityPublic);
  
  // Generate access keypair
  console.log("2️⃣  Generating access keypair...");
  const accessPrivate = ed.utils.randomPrivateKey();
  const accessPublic = await ed.getPublicKeyAsync(accessPrivate);
  const accessPrivateHex = bytesToHex(accessPrivate);
  const accessPublicHex = bytesToHex(accessPublic);
  
  // Register identity
  console.log("3️⃣  Registering identity...");
  let identityId;
  try {
    const res = await fetch(`${url}/identity/register`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ public_key: identityPublicHex }),
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data.error);
    identityId = data.identity_id;
    console.log(`   Identity ID: ${identityId}`);
  } catch (err) {
    console.error(`❌ Failed to register identity: ${err.message}`);
    process.exit(1);
  }
  
  // Create access key (signed by identity)
  console.log("4️⃣  Creating access key...");
  let accessId;
  try {
    const timestamp = Date.now().toString();
    const message = `create_access:${accessPublicHex}:${timestamp}`;
    const messageBytes = new TextEncoder().encode(message);
    const signature = await ed.signAsync(messageBytes, identityPrivate);
    
    const res = await fetch(`${url}/access/create`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        identity_id: identityId,
        access_public_key: accessPublicHex,
        timestamp,
        signature: bytesToHex(signature),
        name: name || "cli",
      }),
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data.error);
    accessId = data.access_id;
    console.log(`   Access ID: ${accessId}`);
  } catch (err) {
    console.error(`❌ Failed to create access key: ${err.message}`);
    process.exit(1);
  }
  
  // Save credentials
  const credentials = {
    hub_url: url,
    identity_id: identityId,
    identity_public_key: identityPublicHex,
    identity_private_key: identityPrivateHex,
    access_id: accessId,
    access_public_key: accessPublicHex,
    access_private_key: accessPrivateHex,
    created_at: new Date().toISOString(),
  };
  
  // Ensure directory exists
  const dir = dirname(output);
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true });
  }
  
  writeFileSync(output, JSON.stringify(credentials, null, 2));
  console.log(`\n✅ Credentials saved to: ${output}`);
  console.log("\n⚠️  IMPORTANT: Keep your identity_private_key secure!");
  console.log("   It proves ownership of your contracts.");
}

function bytesToHex(bytes) {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, "0")).join("");
}
