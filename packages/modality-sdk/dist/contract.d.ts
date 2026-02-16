import { type Identity } from './identity.js';
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
export declare function hashJSON(data: unknown): string;
/** Create a new contract from an intent template */
export declare function createContract(intent: IntentTemplate, creator: Identity): ContractGenesis;
/** Verify a contract genesis signature */
export declare function verifyGenesis(genesis: ContractGenesis): boolean;
/** Create a new commit */
export declare function createCommit(contractId: string, sequence: number, actions: CommitAction[], parentHash: string | null): Commit;
/** Sign a commit with an identity */
export declare function signCommit(commit: Commit, identity: Identity): Commit;
/** Verify all signatures on a commit */
export declare function verifyCommit(commit: Commit): boolean;
/** Get the hash of a commit (for chaining) */
export declare function commitHash(commit: Commit): string;
