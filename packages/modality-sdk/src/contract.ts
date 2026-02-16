import { sha256 } from '@noble/hashes/sha256';
import { bytesToHex } from '@noble/hashes/utils';
import { signJSON, verifyJSON, type Identity } from './identity.js';
import type { IntentTemplate } from './templates.js';

export interface ContractGenesis {
  contract_id: string;
  created_at: string;
  intent: IntentTemplate;
  signature: string;
  signer: string;
}

export interface CommitAction {
  method: 'post' | 'rule';
  path: string | null;
  value: unknown;
}

export interface Commit {
  contract_id: string;
  sequence: number;
  body: CommitAction[];
  head: {
    parent_hash: string | null;
    signatures: Record<string, string>;
    [key: string]: unknown;
  };
}

/** Hash a JSON object deterministically */
export function hashJSON(data: unknown): string {
  const bytes = new TextEncoder().encode(JSON.stringify(data));
  return bytesToHex(sha256(bytes));
}

/** Create a new contract from an intent template */
export function createContract(
  intent: IntentTemplate,
  creator: Identity,
): ContractGenesis {
  const created_at = new Date().toISOString();
  const contract_id = hashJSON({ intent, created_at, creator: creator.publicKey });
  const payload = { contract_id, created_at, intent };
  const signature = signJSON(payload, creator.privateKey);
  return {
    contract_id,
    created_at,
    intent,
    signature,
    signer: creator.publicKey,
  };
}

/** Verify a contract genesis signature */
export function verifyGenesis(genesis: ContractGenesis): boolean {
  const { signature, signer, ...payload } = genesis;
  return verifyJSON(payload, signature, signer);
}

/** Create a new commit */
export function createCommit(
  contractId: string,
  sequence: number,
  actions: CommitAction[],
  parentHash: string | null,
): Commit {
  return {
    contract_id: contractId,
    sequence,
    body: actions,
    head: {
      parent_hash: parentHash,
      signatures: {},
    },
  };
}

/** Sign a commit with an identity */
export function signCommit(commit: Commit, identity: Identity): Commit {
  const sig = signJSON(commit.body, identity.privateKey);
  return {
    ...commit,
    head: {
      ...commit.head,
      signatures: {
        ...commit.head.signatures,
        [identity.publicKey]: sig,
      },
    },
  };
}

/** Verify all signatures on a commit */
export function verifyCommit(commit: Commit): boolean {
  for (const [pubkey, sig] of Object.entries(commit.head.signatures)) {
    if (!verifyJSON(commit.body, sig, pubkey)) return false;
  }
  return Object.keys(commit.head.signatures).length > 0;
}

/** Get the hash of a commit (for chaining) */
export function commitHash(commit: Commit): string {
  return hashJSON(commit);
}
