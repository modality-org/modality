import fs from "fs";
import path from "path";

export const command = "predicate";
export const desc = "Manage and test WASM predicates";

export const builder = (yargs) => {
  return yargs
    .command(
      "list [contract-id]",
      "List predicates available in a contract or network",
      (yargs) => {
        return yargs.positional("contract-id", {
          describe: "Contract ID (defaults to modal.money for network predicates)",
          type: "string",
          default: "modal.money",
        });
      },
      async (argv) => {
        const { listPredicates } = await import("./predicate/list-predicate.js");
        await listPredicates(argv);
      }
    )
    .command(
      "info <name>",
      "Get information about a specific predicate",
      (yargs) => {
        return yargs
          .positional("name", {
            describe: "Predicate name (e.g., amount_in_range)",
            type: "string",
          })
          .option("contract-id", {
            describe: "Contract ID (defaults to modal.money)",
            type: "string",
            default: "modal.money",
          });
      },
      async (argv) => {
        const { predicateInfo } = await import("./predicate/info-predicate.js");
        await predicateInfo(argv);
      }
    )
    .command(
      "test <name>",
      "Test a predicate with sample data",
      (yargs) => {
        return yargs
          .positional("name", {
            describe: "Predicate name",
            type: "string",
          })
          .option("args", {
            describe: "Arguments as JSON string",
            type: "string",
            demandOption: true,
          })
          .option("contract-id", {
            describe: "Contract ID",
            type: "string",
            default: "modal.money",
          })
          .option("block-height", {
            describe: "Block height for context",
            type: "number",
            default: 1,
          })
          .option("timestamp", {
            describe: "Timestamp for context",
            type: "number",
            default: Date.now(),
          });
      },
      async (argv) => {
        const { testPredicate } = await import("./predicate/test-predicate.js");
        await testPredicate(argv);
      }
    )
    .command(
      "upload <wasm-file>",
      "Upload a custom predicate to a contract",
      (yargs) => {
        return yargs
          .positional("wasm-file", {
            describe: "Path to WASM file",
            type: "string",
          })
          .option("contract-id", {
            describe: "Contract ID",
            type: "string",
            demandOption: true,
          })
          .option("name", {
            describe: "Predicate name (inferred from filename if not specified)",
            type: "string",
          })
          .option("gas-limit", {
            describe: "Gas limit for execution",
            type: "number",
            default: 10000000,
          });
      },
      async (argv) => {
        const { uploadPredicate } = await import("./predicate/upload-predicate.js");
        await uploadPredicate(argv);
      }
    )
    .demandCommand(1, "Please specify a predicate command")
    .help();
};

export const handler = () => {};

