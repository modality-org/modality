/**
 * modal hub stop - Stop the contract hosting service
 */

import { existsSync, readFileSync, unlinkSync } from "fs";
import { join } from "path";

export const command = "stop";
export const describe = "Stop the contract hosting service";

export function builder(yargs) {
  return yargs
    .option("data-dir", {
      alias: "d",
      type: "string",
      default: "./.modal-hub",
      describe: "Data directory where hub is running",
    });
}

export async function handler(argv) {
  const { dataDir } = argv;
  const pidFile = join(dataDir, "hub.pid");
  
  if (!existsSync(pidFile)) {
    console.log("⚠️  No running hub found (no PID file)");
    return;
  }
  
  const pid = parseInt(readFileSync(pidFile, "utf8").trim(), 10);
  
  try {
    process.kill(pid, "SIGTERM");
    console.log(`✅ Stopped Contract Hub (PID: ${pid})`);
    unlinkSync(pidFile);
  } catch (err) {
    if (err.code === "ESRCH") {
      console.log("⚠️  Process not running, cleaning up PID file");
      unlinkSync(pidFile);
    } else {
      console.error("❌ Failed to stop:", err.message);
      process.exit(1);
    }
  }
}
