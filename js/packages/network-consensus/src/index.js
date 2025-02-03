import Runner from "./Runner.js";

import { SEQUENCING_METHODS } from "./sequencing/index.js";
import { ELECTION_METHODS } from "./election/index.js";

export async function setupNetworkConsensus({
  datastore,
  sequencing_method,
  election_method,
  peerid,
  keypair
}) {
  const election = await ELECTION_METHODS[election_method].create();
  const sequencing = await SEQUENCING_METHODS[sequencing_method].create({
    datastore,
    peerid,
    keypair,
    election,
  });
  const consensus_system  = Runner.create({
    datastore,
    peerid,
    keypair,
    sequencing,
  });
  return consensus_system;
}


export async function setupSequencing({
  datastore,
  sequencing_method,
  election_method,
}) {
  const election = ELECTION_METHODS[election_method].create();
  const sequencing = SEQUENCING_METHODS[sequencing_method].create({
    datastore,
    election,
  });
  return sequencing;
}