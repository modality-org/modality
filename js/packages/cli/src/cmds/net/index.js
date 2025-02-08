import * as ping from "./ping.js";
import * as run_node from "./run_node.js";
import * as genesis from "./genesis.js";

export const command = "net <cmd>";
export const describe = "Modality Network related commands";
export const aliases = ["network"];
export const builder = (yargs) =>
  yargs
    .demandCommand(1)
    .command(ping)
    .command(run_node)
    .command(genesis)
    .demandCommand(1)
    .help();
