import type { ContractGenesis } from './contract.js';
export interface ContractCard {
    '@atp': '1.0';
    contract_id: string;
    summary: string;
    parties: Record<string, {
        id: string;
        role: string;
    }>;
    my_role: string;
    my_rights: string[];
    my_obligations: string[];
    my_protections: string[];
    current_state: string;
    available_actions: string[];
}
/** Generate a Contract Card for a specific party */
export declare function generateCard(genesis: ContractGenesis, myPublicKey: string): ContractCard;
