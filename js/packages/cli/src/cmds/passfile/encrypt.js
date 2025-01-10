export const command = 'encrypt';
export const describe = 'Encrypt Modality passfile in place';
export const builder = {
  path: {
    type: 'filepath',
  }
};

import Keypair from "@modality-dev/utils/Keypair";
import fs from 'fs-extra';
import inquirer from 'inquirer';

export async function handler({ path }) {
  const keypair = await Keypair.fromJSONFile(path);
  await keypair.asJSONFile(path);
  const password = await getPassword();
  await keypair.asEncryptedJSONFile(path, password);
  console.log("🔐 Encrypted")
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