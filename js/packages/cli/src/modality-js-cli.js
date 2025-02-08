#!/usr/bin/env node

import yargs from "yargs";
import { hideBin } from "yargs/helpers";

import { commands } from "./cmds/index.js";

// unused return variable prevents node from prematurely exiting yargs
/* eslint-disable no-unused-vars */
const { argv } = yargs(hideBin(process.argv))
  .scriptName("modality-js")
  .help("h")
  .alias("h", "help")
  .wrap(null)
  .command(commands)
  .demandCommand(1, "command not recognized")
  .epilogue("for more information, view the docs at https://www.modality.org/")
  .strict();
