// see yargs docs
export const command = "genesis";
export const describe = "Generate or fork a network";
export const builder = {
  passfile: {
    type: "filepath",
    demandOption: true,
  },
  dir: {
    type: "filepath",
    demandOption: true,
  },
};

import Keypair from "@modality-dev/utils/Keypair";
import fs from "fs-extra";
import Block from "@modality-dev/network-datastore/data/Block";

export async function handler({ passfile, dir }) {
  const keypair = await Keypair.fromJSONFile(passfile);
  const peer_id = await keypair.asPublicAddress();
  fs.ensureDirSync(dir);
  if (!fs.existsSync(`${dir}/setup`)) {
    console.log("Existing ./setup directory not found");
    console.log("Creating Round 0 with no events");
    console.log(`If you need events within your genesis:
  1) edit them here: ${dir}/setup/rounds/0/${peer_id}/events.json
  2) delete the dir: ${dir}/completed/rounds/0
  3) rerun genesis
    `);
    fs.ensureDirSync(`${dir}/setup/rounds/0/${peer_id}`);
    fs.writeJSONSync(`${dir}/setup/rounds/0/${peer_id}/events.json`, []);
  }

  let network_config = { rounds: {} };
  const rounds = fs.readdirSync(`${dir}/setup/rounds/`);
  let all_blocks_have_certs = true;
  for (const round_id of rounds) {
    // if (fs.existsSync(`${dir}/completed/rounds/${round_id}/certified.json`)) {
    //   console.log(`Skipping already completed round ${round_id}`);
    //   continue;
    // }
    const my_peer_id = peer_id;
    const peers = fs.readdirSync(`${dir}/setup/rounds/${round_id}`);
    network_config.rounds[round_id] = {};
    for (const peer_id of peers) {
      let block;
      if (
        peer_id === my_peer_id &&
        fs.existsSync(
          `${dir}/setup/rounds/${round_id}/${peer_id}/events.json`
        ) &&
        !fs.existsSync(`${dir}/setup/rounds/${round_id}/${peer_id}/block.json`)
      ) {
        console.log(`CREATE BLOCK round_id: ${round_id}, peer_id: ${peer_id}`);
        const events = fs.readJSONSync(
          `${dir}/setup/rounds/${round_id}/${peer_id}/events.json`
        );
        block = Block.from({
          round_id,
          peer_id,
          last_round_certs: [],
          events,
        });
        await block.generateOpeningSig(keypair);
        await block.generateClosingSig(keypair);
        const ack = await block.generateAck(keypair);
        await block.addAck(ack);
        fs.writeJSONSync(
          `${dir}/setup/rounds/${round_id}/${peer_id}/block.json`,
          block.toJSONObject(),
          { spaces: 2 }
        );
      } else if (
        fs.existsSync(`${dir}/setup/rounds/${round_id}/${peer_id}/block.json`)
      ) {
        console.log(`ACK round_id: ${round_id}, peer_id: ${peer_id}`);
        const block_data = fs.readJSONSync(
          `${dir}/setup/rounds/${round_id}/${peer_id}/block.json`
        );
        block = Block.from(block_data);
        const ack = await block.generateAck(keypair);
        await block.addAck(ack);
        fs.writeJSONSync(
          `${dir}/setup/rounds/${round_id}/${peer_id}/block.json`,
          block.toJSONObject(),
          { spaces: 2 }
        );
      }
      if (peer_id === my_peer_id) {
        try {
          await block.generateCert(keypair);
          fs.writeJSONSync(
            `${dir}/setup/rounds/${round_id}/${peer_id}/block.json`,
            block.toJSONObject(),
            { spaces: 2 }
          );
        } catch (e) {
          //
        }
      }
      if (block?.cert) {
        network_config.rounds[round_id][peer_id] = block.toJSONObject();
      } else {
        console.log(`MISSING CERT round_id: ${round_id}, peer_id: ${peer_id}`);
        all_blocks_have_certs = false;
      }
    }
  }
  if (all_blocks_have_certs) {
    console.log(`Outputting network config to ${dir}/config.json`);
    if (fs.existsSync(`${dir}/setup/info.json`)) {
      const info = fs.readJsonSync(`${dir}/setup/info.json`);
      network_config = { ...info, ...network_config };
    }
    fs.writeJSONSync(`${dir}/config.json`, network_config, { spaces: 2 });
  } else {
    console.log(`Not all blocks have certs, have the other nodes sign`);
  }
}

export default handler;

// so we can directly test the file
import cliCalls from "cli-calls";
await cliCalls(import.meta, handler);
