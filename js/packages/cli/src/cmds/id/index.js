import * as create from './create.js';

export const command = [
  create,
].map(cmd => ({
  ...cmd,
  command: typeof cmd.command === "string" ? `id ${cmd.command}` : cmd.command
}));