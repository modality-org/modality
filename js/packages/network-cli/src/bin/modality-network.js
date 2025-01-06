#!/usr/bin/env node

import yargs from "yargs";
import { hideBin } from "yargs/helpers";

import * as create_mock_datastore from "../commands/create_mock_datastore.js";

// unused return variable prevents node from prematurely exiting yargs
/* eslint-disable no-unused-vars */
const { argv } = yargs(hideBin(process.argv))
  .scriptName("modality-network")
  .help("h")
  .alias("h", "help")
  .wrap(null)
  .demandCommand(1, "command not recognized")
  .command(create_mock_datastore)
  .epilogue("for more information, view the docs at https://www.modality.dev/")
  .strict();
