import Runner from "./Runner";

import { SEQUENCING_METHODS } from "./sequencing";
import { ELECTION_METHODS } from "./election";

export async function setupNetworkConsensus({
  datastore,
  sequencing_method,
  election_method,
  peerid,
  keypair
}) {
  const election = ELECTION_METHODS[election_method].create();
  const sequencing = SEQUENCING_METHODS[sequencing_method].create({
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