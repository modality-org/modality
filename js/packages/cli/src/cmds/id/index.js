import * as create from './create.js';

export const command = 'id <cmd>';
export const describe = 'Modality ID related commands';
export const builder = (yargs) => yargs.demandCommand(1)
  .command(create)
  ;