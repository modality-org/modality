export const command = 'create';
export const describe = 'Create a new Modality ID and associated passfile';
export const builder = {
  path: {
    type: 'filepath',
  },
  encrypt: {
    type: 'bool',
  }
};

import Keypair from "@modality-dev/utils/Keypair";
import fs from 'fs-extra';
import inquirer from 'inquirer';

export async function handler({path, encrypt}) {

  const keypair = await Keypair.generate();
  let address = await keypair.asPublicAddress();
  if (!path) {
    path = `./${address}.mod_passfile`;
  }
  if (fs.existsSync(path)) {
    throw new Error('passfile already exists') 
  }

  if (encrypt) {
    const password = await getPassword();
    await keypair.asEncryptedJSONFile(path, password); 
  } else {
    await keypair.asJSONFile(path); 
  }

  console.log("✨ Successfully created a new Modality ID!");
  console.log("📍 Modality ID: %s", address);
  console.log("🔑 Modality Passfile saved to: %s", path);
  console.log("\n🚨🚨🚨  IMPORTANT: Keep your passkey file secure and never share it! 🚨🚨🚨");
}

export default handler;

async function getPassword() {
  const { password } = await inquirer.prompt([
    {
      type: 'password',
      name: 'password',
      message: 'Enter password to encrypt the passfile:',
      mask: '*'
    }
  ]);

  if (password.length === 0) {
    return { error: new Error('Password cannot be empty') };
  }

  const { confirm } = await inquirer.prompt([
    {
      type: 'password',
      name: 'confirm',
      message: 'Confirm password:',
      mask: '*'
    }
  ]);

  if (password !== confirm) {
    throw new Error('Passwords do not match');
  }

  return password;
}

import cliCalls from "cli-calls";
await cliCalls(import.meta, handler);