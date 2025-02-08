export const command = "decrypt";
export const describe = "Decrypt Modality passfile in place";
export const builder = {
  path: {
    type: "filepath",
  },
};

import Keypair from "@modality-dev/utils/Keypair";
import fs from "fs-extra";
import inquirer from "inquirer";

export async function handler({ path }) {
  const password = await getPassword();
  const keypair = await Keypair.fromEncryptedJSONFile(path, password);
  await keypair.asJSONFile(path);
  console.log("🔓 Decrypted");
}

export default handler;

async function getPassword() {
  const { password } = await inquirer.prompt([
    {
      type: "password",
      name: "password",
      message: "Enter password to decrypt the passfile:",
      mask: "*",
    },
  ]);
  return password;
}

import cliCalls from "cli-calls";
await cliCalls(import.meta, handler);
