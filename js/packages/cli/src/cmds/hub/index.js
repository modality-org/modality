/**
 * Hub commands for contract hosting service
 */

export const command = "hub <command>";
export const describe = "Contract hosting service";

export function builder(yargs) {
  return yargs
    .commandDir(".", {
      extensions: ["js"],
      visit: (cmd) => cmd,
    })
    .demandCommand(1, "You need to specify a hub command")
    .strict();
}

export function handler() {}
