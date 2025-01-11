import * as ping from './ping.js';

export const command = 'net <cmd>';
export const describe = 'Modality Network related commands';
export const builder = (yargs) => yargs.demandCommand(1)
  .command(ping)
  .demandCommand(1)
  .help()
  ;