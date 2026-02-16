const PATTERN_ROLES = {
    escrow: {
        buyer: {
            rights: [
                'Deposit funds to start escrow',
                'Release funds after delivery',
                'Dispute within dispute window',
            ],
            obligations: ['Must release or dispute within dispute window of delivery'],
            protections: [
                'Seller cannot take funds without delivering',
                'Arbiter resolves disputes',
            ],
            initial_actions: ['DEPOSIT'],
        },
        seller: {
            rights: ['Receive payment upon delivery confirmation', 'Deliver goods/services'],
            obligations: ['Must deliver by deadline'],
            protections: [
                'Payment guaranteed once buyer releases',
                'No clawback after release',
            ],
            initial_actions: ['DELIVER'],
        },
        arbiter: {
            rights: ['Resolve disputes between buyer and seller'],
            obligations: ['Must review disputes fairly'],
            protections: ['Arbitration fee guaranteed'],
            initial_actions: [],
        },
    },
    task_delegation: {
        delegator: {
            rights: ['Define task requirements', 'Review submitted work', 'Accept or reject work'],
            obligations: ['Pay upon accepted completion'],
            protections: ['Payment only on completion', 'Quality review period'],
            initial_actions: ['ASSIGN'],
        },
        worker: {
            rights: ['Receive payment upon task acceptance', 'Clear task definition upfront'],
            obligations: ['Complete task by deadline', 'Submit work for review'],
            protections: [
                'Payment guaranteed once work accepted',
                'Cannot be rejected without stated reason',
            ],
            initial_actions: ['ACCEPT', 'REJECT'],
        },
    },
    data_exchange: {
        provider: {
            rights: ['Receive payment for data', 'Set data format and terms'],
            obligations: ['Deliver data matching description'],
            protections: ['Payment before delivery'],
            initial_actions: ['DELIVER'],
        },
        consumer: {
            rights: ['Receive data as described', 'Request refund if data invalid'],
            obligations: ['Pay agreed price'],
            protections: ['Data quality guaranteed', 'Refund if invalid'],
            initial_actions: ['PAY'],
        },
    },
};
/** Generate a Contract Card for a specific party */
export function generateCard(genesis, myPublicKey) {
    const intent = genesis.intent;
    const pattern = intent.pattern;
    const roles = PATTERN_ROLES[pattern];
    // Find which role this key maps to
    let myRole = '';
    const partiesMap = {};
    for (const [role, id] of Object.entries(intent.parties)) {
        partiesMap[role] = { id, role };
        if (id === myPublicKey)
            myRole = role;
    }
    if (!myRole) {
        throw new Error('Public key not found among contract parties');
    }
    const spec = roles?.[myRole];
    const terms = intent.terms;
    // Build summary
    const summaryParts = {
        escrow: `Escrow: ${terms.amount} ${terms.currency}`,
        task_delegation: `Task: ${terms.task} for ${terms.payment} tokens`,
        data_exchange: `Data exchange: ${terms.data_description} for ${terms.price} ${terms.currency}`,
    };
    return {
        '@atp': '1.0',
        contract_id: genesis.contract_id,
        summary: summaryParts[pattern] ?? `${pattern} contract`,
        parties: partiesMap,
        my_role: myRole,
        my_rights: spec?.rights ?? [],
        my_obligations: spec?.obligations ?? [],
        my_protections: spec?.protections ?? [],
        current_state: 'pending',
        available_actions: spec?.initial_actions ?? [],
    };
}
