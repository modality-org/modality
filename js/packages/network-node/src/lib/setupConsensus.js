import Keypair from "@modality-dev/utils/Keypair";
import { setupNetworkConsensus } from '@modality-dev/network-consensus';
import ConsensusCommunication from "./ConsensusCommunication.js";

export async function setupConsensus(node, conf) {
  const keypair = await Keypair.fromJSON(conf.keypair);
  node.consensus = await setupNetworkConsensus({
    datastore: node.storage.datastore,
    keypair,
    peerid: keypair.id,
    sequencing_method: 'DAGRider',
    election_method: 'RoundRobin'
  });
  node.consensus.communication = new ConsensusCommunication({
    node: node, sequencer: node.storage.sequencer
  });
}
