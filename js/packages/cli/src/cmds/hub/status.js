/**
 * modal hub status - Check contract hosting service status
 */

import { existsSync, readFileSync } from "fs";
import { join } from "path";

export const command = "status";
export const describe = "Check contract hosting service status";

export function builder(yargs) {
  return yargs
    .option("data-dir", {
      alias: "d",
      type: "string",
      default: "./.modal-hub",
      describe: "Data directory to check",
    })
    .option("url", {
      alias: "u",
      type: "string",
      default: "http://localhost:3100",
      describe: "Hub URL to check",
    });
}

export async function handler(argv) {
  const { dataDir, url } = argv;
  
  console.log("📊 Contract Hub Status\n");
  
  // Check PID file
  const pidFile = join(dataDir, "hub.pid");
  if (existsSync(pidFile)) {
    const pid = parseInt(readFileSync(pidFile, "utf8").trim(), 10);
    try {
      process.kill(pid, 0); // Check if process exists
      console.log(`✅ Background process running (PID: ${pid})`);
    } catch {
      console.log(`⚠️  PID file exists but process not running (PID: ${pid})`);
    }
  } else {
    console.log(`ℹ️  No background process (no PID file in ${dataDir})`);
  }
  
  // Check health endpoint
  try {
    const res = await fetch(`${url}/health`);
    const data = await res.json();
    console.log(`✅ Service responding at ${url}`);
    console.log(`   Version: ${data.version || "unknown"}`);
  } catch (err) {
    console.log(`❌ Service not responding at ${url}`);
    console.log(`   Error: ${err.message}`);
  }
  
  // Check data directory
  const dbFile = join(dataDir, "contracts.db");
  if (existsSync(dbFile)) {
    console.log(`✅ Database exists: ${dbFile}`);
  } else {
    console.log(`ℹ️  No database yet (will be created on first request)`);
  }
}
