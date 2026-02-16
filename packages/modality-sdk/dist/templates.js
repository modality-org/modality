/** Intent template patterns for common contract types */
export function escrow(parties, terms, options) {
    return {
        '@atp_intent': '1.0',
        pattern: 'escrow',
        parties,
        terms: terms,
        buyer_protections: options?.buyer_protections ?? [
            'delivery_required',
            'dispute_allowed',
        ],
        seller_protections: options?.seller_protections ?? [
            'payment_guaranteed',
            'no_clawback_after_release',
        ],
    };
}
export function taskDelegation(parties, terms, options) {
    return {
        '@atp_intent': '1.0',
        pattern: 'task_delegation',
        parties,
        terms: terms,
        delegator_protections: options?.delegator_protections ?? [
            'payment_on_completion',
            'quality_review_period',
        ],
        worker_protections: options?.worker_protections ?? [
            'payment_guaranteed_if_accepted',
            'clear_requirements',
        ],
    };
}
export function dataExchange(parties, terms, options) {
    return {
        '@atp_intent': '1.0',
        pattern: 'data_exchange',
        parties,
        terms: terms,
        provider_protections: options?.provider_protections ?? [
            'payment_before_delivery',
        ],
        consumer_protections: options?.consumer_protections ?? [
            'data_quality_guaranteed',
            'refund_if_invalid',
        ],
    };
}
