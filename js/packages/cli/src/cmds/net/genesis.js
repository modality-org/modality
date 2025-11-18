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
import Contract from "@modality-dev/contract/Contract";
import Commit from "@modality-dev/contract/Commit";
import crypto from "crypto";
import path from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Standard predicates to include in network genesis contract
const STANDARD_PREDICATES = [
  { name: "signed_by", description: "Verify cryptographic signatures" },
  { name: "amount_in_range", description: "Check numeric bounds" },
  { name: "has_property", description: "Check JSON property existence" },
  { name: "timestamp_valid", description: "Validate timestamp constraints" },
  { name: "post_to_path", description: "Verify commit actions" },
];

/**
 * Add standard WASM predicates to the genesis contract
 * These predicates are available network-wide at /_code/modal/*.wasm
 */
async function addStandardPredicates(commit) {
  // Path to compiled predicates
  const predicatesDir = path.join(__dirname, "../../../../../build/wasm/predicates");
  
  // Check if predicates directory exists
  if (!fs.existsSync(predicatesDir)) {
    console.log("⚠️  Standard predicates not found. Run build-predicates.sh to compile them.");
    console.log(`   Expected location: ${predicatesDir}`);
    return;
  }
  
  let addedCount = 0;
  
  for (const predicate of STANDARD_PREDICATES) {
    const wasmPath = path.join(predicatesDir, `${predicate.name}.wasm`);
    
    if (fs.existsSync(wasmPath)) {
      const wasmBytes = fs.readFileSync(wasmPath);
      const wasmBase64 = wasmBytes.toString("base64");
      
      // Add predicate as POST action to /_code/modal/{name}.wasm
      commit.addPost(`/_code/modal/${predicate.name}.wasm`, wasmBase64);
      
      console.log(`  ✓ Added predicate: ${predicate.name} (${predicate.description})`);
      addedCount++;
    } else {
      console.log(`  ⚠️  Predicate not found: ${predicate.name}.wasm`);
    }
  }
  
  if (addedCount > 0) {
    console.log(`✓ Added ${addedCount} standard predicates to genesis contract`);
  }
}

async function createNetworkGenesisContract(networkInfo) {
  // Generate a genesis contract for the network parameters
  const genesisData = await Contract.generateGenesis();
  const contractId = genesisData.genesis.contract_id;
  
  // Create a commit with all network parameters as POST actions
  const commit = new Commit();
  
  // Network metadata
  commit.addPost("/network/name.text", networkInfo.name || "network");
  commit.addPost("/network/description.text", networkInfo.description || "");
  
  // Network parameters
  commit.addPost("/network/difficulty.number", String(networkInfo.difficulty || 1));
  commit.addPost("/network/target_block_time_secs.number", String(networkInfo.target_block_time_secs || 60));
  commit.addPost("/network/miner_hash_func.text", networkInfo.miner_hash_func || "randomx");
  
  // Add miner_hash_params if provided
  if (networkInfo.miner_hash_params) {
    commit.addPost("/network/miner_hash_params.json", JSON.stringify(networkInfo.miner_hash_params));
  }
  
  commit.addPost("/network/blocks_per_epoch.number", String(networkInfo.blocks_per_epoch || 40));
  
  // Validators (indexed)
  if (networkInfo.validators && networkInfo.validators.length > 0) {
    networkInfo.validators.forEach((validator, index) => {
      commit.addPost(`/network/validators/${index}.text`, validator);
    });
  }
  
  // Add standard predicates to /_code/modal/ (if available)
  // These are compiled WASM modules that provide standard validation logic
  await addStandardPredicates(commit);
  
  // Note: Bootstrappers are NOT included in the genesis contract
  // They are operational/networking config only, kept in network config file
  
  // Compute commit ID (SHA256 of the commit JSON)
  const commitJson = JSON.stringify({ body: commit.body, head: commit.head });
  const commitId = crypto.createHash('sha256').update(commitJson).digest('hex');
  
  return {
    contractId,
    commitId,
    genesisData,
    commit: { body: commit.body, head: commit.head }
  };
}

export async function handler({ passfile, dir }) {
  const keypair = await Keypair.fromJSONFile(passfile);
  const peer_id = await keypair.asPublicAddress();
  fs.ensureDirSync(dir);
  
  // Load network info from setup/info.json
  let networkInfo = { name: "network", description: "" };
  if (fs.existsSync(`${dir}/setup/info.json`)) {
    networkInfo = { ...networkInfo, ...fs.readJsonSync(`${dir}/setup/info.json`) };
  }
  
  // Create network genesis contract
  console.log("Creating network genesis contract...");
  const networkContract = await createNetworkGenesisContract(networkInfo);
  console.log(`Network contract ID: ${networkContract.contractId}`);
  console.log(`Genesis commit ID: ${networkContract.commitId}`);
  
  // Save the network contract for reference
  fs.ensureDirSync(`${dir}/setup`);
  fs.writeJSONSync(`${dir}/setup/network-contract.json`, {
    contractId: networkContract.contractId,
    genesis: networkContract.genesisData,
    genesisCommit: networkContract.commit
  }, { spaces: 2 });
  
  // Check if events.json exists for this peer
  const eventsPath = `${dir}/setup/rounds/0/${peer_id}/events.json`;
  if (!fs.existsSync(eventsPath)) {
    console.log("Creating Round 0 events.json");
    fs.ensureDirSync(`${dir}/setup/rounds/0/${peer_id}`);
    
    // Create event for network contract genesis commit
    const networkContractEvent = {
      type: "contract-commit",
      contract_id: networkContract.contractId,
      commit_id: networkContract.commitId,
      commit: networkContract.commit
    };
    
    fs.writeJSONSync(eventsPath, [networkContractEvent], { spaces: 2 });
  } else {
    // If events.json exists, prepend the network contract event if not already present
    const existingEvents = fs.readJSONSync(eventsPath);
    
    // Check if network contract event is already in the events
    const hasNetworkContractEvent = existingEvents.some(e => 
      e.type === "contract-commit" && e.contract_id === networkContract.contractId
    );
    
    if (!hasNetworkContractEvent) {
      const networkContractEvent = {
        type: "contract-commit",
        contract_id: networkContract.contractId,
        commit_id: networkContract.commitId,
        commit: networkContract.commit
      };
      
      // Prepend network contract event
      const allEvents = [networkContractEvent, ...existingEvents];
      fs.writeJSONSync(eventsPath, allEvents, { spaces: 2 });
      console.log("Added network contract event to existing events.json");
    } else {
      console.log("Network contract event already exists in events.json");
    }
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
    
    // Add genesis contract ID and latest parameters
    network_config.genesis_contract_id = networkContract.contractId;
    network_config.latest_parameters = {
      difficulty: networkInfo.difficulty || 1,
      target_block_time_secs: networkInfo.target_block_time_secs || 60,
      blocks_per_epoch: networkInfo.blocks_per_epoch || 40,
      validators: networkInfo.validators || [],
      bootstrappers: networkInfo.bootstrappers || []
    };
    
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
