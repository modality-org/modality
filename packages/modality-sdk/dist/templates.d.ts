/** Intent template patterns for common contract types */
export interface IntentTemplate {
    '@atp_intent': '1.0';
    pattern: string;
    parties: Record<string, string>;
    terms: Record<string, unknown>;
    [key: string]: unknown;
}
export interface EscrowTerms {
    amount: number;
    currency: string;
    delivery_deadline: string;
    dispute_window_hours: number;
    [key: string]: unknown;
}
export interface TaskDelegationTerms {
    task: string;
    payment: number;
    deadline: string;
    [key: string]: unknown;
}
export interface DataExchangeTerms {
    data_description: string;
    price: number;
    currency: string;
    format: string;
    [key: string]: unknown;
}
export declare function escrow(parties: {
    buyer: string;
    seller: string;
    arbiter?: string;
}, terms: EscrowTerms, options?: {
    buyer_protections?: string[];
    seller_protections?: string[];
}): IntentTemplate;
export declare function taskDelegation(parties: {
    delegator: string;
    worker: string;
}, terms: TaskDelegationTerms, options?: {
    delegator_protections?: string[];
    worker_protections?: string[];
}): IntentTemplate;
export declare function dataExchange(parties: {
    provider: string;
    consumer: string;
}, terms: DataExchangeTerms, options?: {
    provider_protections?: string[];
    consumer_protections?: string[];
}): IntentTemplate;
