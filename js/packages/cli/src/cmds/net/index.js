import * as ping from "./ping.js";
import * as run_node from "./run_node.js";
import * as genesis from "./genesis.js";
import * as datastore_clear from './datastore_clear.js'
import * as datastore_keys from './datastore_keys.js'
import * as datastore_get from './datastore_get.js'

export const command = "net <cmd>";
export const describe = "Modality Network related commands";
export const aliases = ["network"];
export const builder = (yargs) =>
  yargs
    .demandCommand(1)
    .command(ping)
    .command(run_node)
    .command(genesis)
    .command(datastore_clear)
    .command(datastore_keys)
    .command(datastore_get)
    .demandCommand(1)
    .help();
