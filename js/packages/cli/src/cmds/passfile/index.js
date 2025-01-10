import * as decrypt from './decrypt.js';
import * as encrypt from './encrypt.js';

export const command = 'passfile <cmd>';
export const describe = 'Modality Passfile related commands';
export const builder = (yargs) => yargs.demandCommand(1)
  .command(decrypt)
  .command(encrypt)
  .demandCommand(1)
  .help()
  ;