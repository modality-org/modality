/**
 * Shared utilities for hub commands
 */

import { existsSync, readFileSync } from "fs";
import * as ed from "@noble/ed25519";
import { sha512 } from "@noble/hashes/sha512";

// Configure ed25519
ed.etc.sha512Sync = (...m) => sha512(ed.etc.concatBytes(...m));

export function loadCredentials(path) {
  if (!existsSync(path)) {
    console.error(`❌ Credentials not found: ${path}`);
    console.error("   Run: modal hub register");
    return null;
  }
  
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch (err) {
    console.error(`❌ Invalid credentials file: ${err.message}`);
    return null;
  }
}

export async function createAuthHeaders(creds, method, path, body = null) {
  const timestamp = Date.now().toString();
  const bodyHash = hashBody(body);
  const message = `${method}:${path}:${timestamp}:${bodyHash}`;
  
  const messageBytes = new TextEncoder().encode(message);
  const privateKey = hexToBytes(creds.access_private_key);
  const signature = await ed.signAsync(messageBytes, privateKey);
  
  return {
    "Content-Type": "application/json",
    "X-Access-Id": creds.access_id,
    "X-Timestamp": timestamp,
    "X-Signature": bytesToHex(signature),
  };
}

function hashBody(body) {
  if (!body || (typeof body === "object" && Object.keys(body).length === 0)) {
    return "empty";
  }
  const str = typeof body === "string" ? body : JSON.stringify(body);
  const bytes = new TextEncoder().encode(str);
  const hash = sha512(bytes);
  return bytesToHex(hash.slice(0, 16));
}

export function hexToBytes(hex) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
  }
  return bytes;
}

export function bytesToHex(bytes) {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, "0")).join("");
}
