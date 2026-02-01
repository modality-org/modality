/**
 * modal hub start - Start the contract hosting service
 */

import { spawn } from "child_process";
import { existsSync, mkdirSync, writeFileSync, readFileSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";

export const command = "start";
export const describe = "Start the contract hosting service";

export function builder(yargs) {
  return yargs
    .option("port", {
      alias: "p",
      type: "number",
      default: 3100,
      describe: "Port to listen on",
    })
    .option("data-dir", {
      alias: "d",
      type: "string",
      default: "./.modal-hub",
      describe: "Data directory for storage",
    })
    .option("detach", {
      type: "boolean",
      default: false,
      describe: "Run in background",
    });
}

export async function handler(argv) {
  const { port, dataDir, detach } = argv;
  
  // Ensure data directory exists
  if (!existsSync(dataDir)) {
    mkdirSync(dataDir, { recursive: true });
    console.log(`📁 Created data directory: ${dataDir}`);
  }
  
  // Find the contract-hub service
  const __dirname = dirname(fileURLToPath(import.meta.url));
  const hubPath = join(__dirname, "../../../../../..", "services/contract-hub");
  const serverPath = join(hubPath, "src/server.js");
  
  if (!existsSync(serverPath)) {
    console.error("❌ Contract Hub service not found at:", hubPath);
    console.error("   Run from the modality repository root");
    process.exit(1);
  }
  
  // Check if node_modules exists
  const nodeModulesPath = join(hubPath, "node_modules");
  if (!existsSync(nodeModulesPath)) {
    console.log("📦 Installing dependencies...");
    const install = spawn("npm", ["install"], {
      cwd: hubPath,
      stdio: "inherit",
    });
    
    await new Promise((resolve, reject) => {
      install.on("close", (code) => {
        if (code === 0) resolve();
        else reject(new Error(`npm install failed with code ${code}`));
      });
    });
  }
  
  console.log(`🔐 Starting Contract Hub on port ${port}...`);
  console.log(`   Data directory: ${dataDir}`);
  
  const env = {
    ...process.env,
    PORT: port.toString(),
    DATA_DIR: dataDir,
  };
  
  if (detach) {
    // Run in background
    const child = spawn("node", [serverPath], {
      cwd: hubPath,
      env,
      detached: true,
      stdio: "ignore",
    });
    
    child.unref();
    
    // Save PID for stop command
    const pidFile = join(dataDir, "hub.pid");
    writeFileSync(pidFile, child.pid.toString());
    
    console.log(`✅ Contract Hub started in background (PID: ${child.pid})`);
    console.log(`   Stop with: modal hub stop --data-dir ${dataDir}`);
    console.log(`   URL: http://localhost:${port}`);
  } else {
    // Run in foreground
    const child = spawn("node", [serverPath], {
      cwd: hubPath,
      env,
      stdio: "inherit",
    });
    
    child.on("close", (code) => {
      process.exit(code);
    });
    
    // Handle Ctrl+C
    process.on("SIGINT", () => {
      child.kill("SIGINT");
    });
  }
}
