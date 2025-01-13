// see yargs docs
export const command = 'genesis';
export const describe = 'Generate or fork a network';
export const builder = {
  passfile: {},
  dir: {}
};

import Keypair from "@modality-dev/utils/Keypair";
import fs from 'fs-extra';
import Block from '@modality-dev/network-datastore/data/Block';

export async function handler({passfile, dir}) {
  const keypair = await Keypair.fromJSONFile(passfile);
  const peer_id = await keypair.asPublicAddress();
  fs.ensureDirSync(dir);
  if (!fs.existsSync(`${dir}/setup`)) {
    console.log("Existing ./setup directory not found")
    console.log("Creating Round 0 with no events");
    console.log(`If you need events within your genesis:
  1) edit them here: ${dir}/setup/rounds/0/${peer_id}/events.json
  2) delete the dir: ${dir}/completed/rounds/0
  3) rerun genesis
    `);
    fs.ensureDirSync(`${dir}/setup/rounds/0/${peer_id}`);
    fs.writeJSONSync(`${dir}/setup/rounds/0/${peer_id}/events.json`, []);
  }

  const network_config = {rounds: {}};
  const rounds = fs.readdirSync(`${dir}/setup/rounds/`);
  for (const round_id of rounds) {
    // if (fs.existsSync(`${dir}/completed/rounds/${round_id}/certified.json`)) {
    //   console.log(`Skipping already completed round ${round_id}`);
    //   continue;
    // }
    const my_peer_id = peer_id;
    const peers = fs.readdirSync(`${dir}/setup/rounds/${round_id}`);
    for (const peer_id of peers) {
      if (peer_id === my_peer_id) {
        const events = fs.readJSONSync(`${dir}/setup/rounds/${round_id}/${peer_id}/events.json`);
        const block = Block.from({
          round_id,
          peer_id,
          last_round_certs: [],
          events,
        });
        await block.generateOpeningSig(keypair);
        await block.generateClosingSig(keypair);
        const ack = await block.generateAck(keypair);
        await block.addAck(ack);
        if (peers.length === 1) {
          await block.generateCert(keypair);
          network_config.rounds[round_id] = {[peer_id]: block.toJSONObject()}
        }
        fs.writeJSONSync(`${dir}/setup/rounds/${round_id}/${peer_id}/block.json`, block.toJSONObject());
      }
    }
  }
  console.log(`Outputting network config to ${dir}/network-config.json`);
  fs.writeJSONSync(`${dir}/network-config.json`, network_config, {spaces: 2} );
}

export default handler;

// so we can directly test the file
import cliCalls from "cli-calls";
await cliCalls(import.meta, handler);