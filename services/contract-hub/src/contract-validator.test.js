import test from 'node:test';
import assert from 'node:assert/strict';

import { ContractValidator, validateContractLogic } from './contract-validator.js';

test('MODEL commits load predicate-guarded models that validate POST commits', () => {
  const validator = new ContractValidator();

  validator.applyCommit({
    data: {
      method: 'POST',
      path: '/members/alice.id',
      content: 'alice-key'
    }
  });

  validator.applyCommit({
    data: {
      method: 'MODEL',
      path: '/rules/members.modality',
      content: `
        model members_only {
          initial active
          active -> active [+any_signed(/members) -modifies(/members)]
          active -> active [+modifies(/members) +all_signed(/members)]
        }
      `
    }
  });

  assert.deepEqual(validator.validateCommit({
    data: {
      method: 'POST',
      path: '/docs/readme.md',
      content: 'ok',
      signatures: [{ signer_key: 'alice-key' }]
    }
  }).ok, true);

  assert.equal(validator.validateCommit({
    data: {
      method: 'POST',
      path: '/docs/readme.md',
      content: 'no signature'
    }
  }).ok, false);
});

test('MODEL replacements must preserve predicates required by existing rules', () => {
  const validator = new ContractValidator();

  validator.applyCommit({
    data: {
      method: 'RULE',
      path: '/rules/signed.modality',
      content: 'rule signed { formula { always (+any_signed(/members)) } }',
      model: `
        model signed_witness {
          initial active
          active -> active [+any_signed(/members)]
        }
      `
    }
  });

  assert.throws(
    () => validator.applyCommit({
      data: {
        method: 'MODEL',
        path: '/rules/open.modality',
        content: `
          model open {
            initial active
            active -> active []
          }
        `
      }
    }),
    /does not satisfy existing rule predicate/
  );
});

test('MODEL replacements may satisfy predicate disjunctions one branch at a time', () => {
  const validator = new ContractValidator();

  validator.applyCommit({
    data: {
      method: 'RULE',
      path: '/rules/alice-or-bob.modality',
      content: 'rule signed { formula { always (+signed_by(/members/alice.id) | +signed_by(/members/bob.id)) } }',
      model: `
        model signed_witness {
          initial active
          active -> active [+signed_by(/members/alice.id)]
        }
      `
    }
  });

  assert.doesNotThrow(() => validator.applyCommit({
    data: {
      method: 'MODEL',
      path: '/rules/bob.modality',
      content: `
        model bob {
          initial active
          active -> active [+signed_by(/members/bob.id)]
        }
      `
    }
  }));

  assert.throws(
    () => validator.loadModel('/rules/open.modality', `
      model open {
        initial active
        active -> active []
      }
    `),
    /does not satisfy existing rule predicate/
  );
});

test('real formula parser extracts parseable rule predicate clauses', () => {
  const validator = new ContractValidator();

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule signed { formula { always (+signed_by(/members/alice.id) or +signed_by(/members/bob.id)) } }'
    ),
    [
      [{ sign: '+', name: 'signed_by', args: ['/members/alice.id'] }],
      [{ sign: '+', name: 'signed_by', args: ['/members/bob.id'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule no_alice { formula { always (not +signed_by(/members/alice.id)) } }'
    ),
    [
      [{ sign: '-', name: 'signed_by', args: ['/members/alice.id'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule docs { formula { always (+modifies(/docs) and +signed_by(/members/alice.id)) } }'
    ),
    [
      [
        { sign: '+', name: 'modifies', args: ['/docs'] },
        { sign: '+', name: 'signed_by', args: ['/members/alice.id'] }
      ]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule docs { formula { when +modifies(/docs) also +signed_by(/members/alice.id) } }'
    ),
    [
      [{ sign: '-', name: 'modifies', args: ['/docs'] }],
      [{ sign: '+', name: 'signed_by', args: ['/members/alice.id'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule docs { formula { when +modifies(/docs) next +signed_by(/members/alice.id) } }'
    ),
    [
      [{ sign: '-', name: 'modifies', args: ['/docs'] }],
      [{ sign: '+', name: 'signed_by', args: ['/members/alice.id'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule transfer_owner { formula { always (when +TRANSFER also signed_by(/owner.id)) } }'
    ),
    [
      [{ sign: '-', name: 'TRANSFER', args: [] }],
      [{ sign: '+', name: 'signed_by', args: ['/owner.id'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule delegated_quorum { formula { always (when (signed_by(/owner.id) or signed_by(/delegate.id)) also threshold(2, /members)) } }'
    ),
    [
      [
        { sign: '-', name: 'signed_by', args: ['/owner.id'] },
        { sign: '-', name: 'signed_by', args: ['/delegate.id'] }
      ],
      [{ sign: '+', name: 'threshold', args: ['2', '/members'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule next_transfer_owner { formula { always (when +TRANSFER next signed_by(/owner.id)) } }'
    ),
    [
      [{ sign: '-', name: 'TRANSFER', args: [] }],
      [{ sign: '+', name: 'signed_by', args: ['/owner.id'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule next_delegated_quorum { formula { always (when (signed_by(/owner.id) or signed_by(/delegate.id)) next threshold(2, /members)) } }'
    ),
    [
      [
        { sign: '-', name: 'signed_by', args: ['/owner.id'] },
        { sign: '-', name: 'signed_by', args: ['/delegate.id'] }
      ],
      [{ sign: '+', name: 'threshold', args: ['2', '/members'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule no_docs { formula { when +modifies(/docs) also false } }'
    ),
    [
      [{ sign: '-', name: 'modifies', args: ['/docs'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule ok_docs { formula { when +modifies(/docs) also true } }'
    ),
    []
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule no_docs { formula { when +modifies(/docs) next false } }'
    ),
    [
      [{ sign: '-', name: 'modifies', args: ['/docs'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule ok_docs { formula { when +modifies(/docs) next true } }'
    ),
    []
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule owner { formula { when true also signed_by(/owner.id) } }'
    ),
    [
      [{ sign: '+', name: 'signed_by', args: ['/owner.id'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule ok { formula { when false also signed_by(/owner.id) } }'
    ),
    []
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule owner { formula { when true next signed_by(/owner.id) } }'
    ),
    [
      [{ sign: '+', name: 'signed_by', args: ['/owner.id'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule ok { formula { when false next signed_by(/owner.id) } }'
    ),
    []
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule owner { formula { always (signed_by(/owner.id)) } }'
    ),
    [
      [{ sign: '+', name: 'signed_by', args: ['/owner.id'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule transfer { formula { always (+TRANSFER) } }'
    ),
    [
      [{ sign: '+', name: 'TRANSFER', args: [] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule transfer { formula { always ([+TRANSFER] signed_by(/owner.id)) } }'
    ),
    [
      [{ sign: '-', name: 'TRANSFER', args: [] }],
      [{ sign: '+', name: 'signed_by', args: ['/owner.id'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule transfer { formula { always ([+TRANSFER -RECV] signed_by(/owner.id)) } }'
    ),
    [
      [{ sign: '-', name: 'TRANSFER', args: [] }],
      [{ sign: '+', name: 'RECV', args: [] }],
      [{ sign: '+', name: 'signed_by', args: ['/owner.id'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule no_release { formula { always ([+RELEASE] false) } }'
    ),
    [
      [{ sign: '-', name: 'RELEASE', args: [] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule no_reject { formula { always ([-RELEASE] false) } }'
    ),
    [
      [{ sign: '+', name: 'RELEASE', args: [] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule tautology { formula { always ([+RELEASE] true) } }'
    ),
    []
  );

  assert.deepEqual(
    validator.extractRulePredicateClauses(
      'rule tautology { formula { always ([+RELEASE] true) } }'
    ),
    []
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule tautology { formula { always (true) } }'
    ),
    []
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule tautology { formula { always (not false) } }'
    ),
    []
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule maybe_release { formula { always (not [+RELEASE] false) } }'
    ),
    [
      [{ sign: '+', name: 'RELEASE', args: [] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule no_release { formula { always (not <+RELEASE> true) } }'
    ),
    [
      [{ sign: '-', name: 'RELEASE', args: [] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule tautology { formula { always (not <+RELEASE> false) } }'
    ),
    []
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule impossible { formula { always (not [+RELEASE] true) } }'
    ),
    [
      [{ sign: '+', name: '__unsatisfiable_rule__!', args: [] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule impossible { formula { always (false) } }'
    ),
    [
      [{ sign: '+', name: '__unsatisfiable_rule__!', args: [] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule impossible { formula { always (not true) } }'
    ),
    [
      [{ sign: '+', name: '__unsatisfiable_rule__!', args: [] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule impossible { formula { always (false and signed_by(/owner.id)) } }'
    ),
    [
      [{ sign: '+', name: '__unsatisfiable_rule__!', args: [] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule owner { formula { always (false or signed_by(/owner.id)) } }'
    ),
    [
      [{ sign: '+', name: 'signed_by', args: ['/owner.id'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule impossible { formula { always (false or false) } }'
    ),
    [
      [{ sign: '+', name: '__unsatisfiable_rule__!', args: [] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule delivery { formula { always (oracle_attests(/oracles/delivery.id, "delivered", "true")) } }'
    ),
    [
      [{
        sign: '+',
        name: 'oracle_attests',
        args: ['/oracles/delivery.id', '"delivered"', '"true"']
      }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule quorum { formula { always (threshold(2, /members)) } }'
    ),
    [
      [{ sign: '+', name: 'threshold', args: ['2', '/members'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule release { formula { always (<+RELEASE> signed_by(/owner.id)) } }'
    ),
    [[
      { sign: '+', name: 'RELEASE', args: [] },
      { sign: '+', name: 'signed_by', args: ['/owner.id'] }
    ]]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule delegated_release { formula { always (<+RELEASE> (signed_by(/owner.id) or signed_by(/delegate.id))) } }'
    ),
    [
      [
        { sign: '+', name: 'RELEASE', args: [] },
        { sign: '+', name: 'signed_by', args: ['/owner.id'] }
      ],
      [
        { sign: '+', name: 'RELEASE', args: [] },
        { sign: '+', name: 'signed_by', args: ['/delegate.id'] }
      ]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule quorum_transfer { formula { always ([+TRANSFER] (signed_by(/owner.id) and threshold(2, /members))) } }'
    ),
    [
      [{ sign: '-', name: 'TRANSFER', args: [] }],
      [
        { sign: '+', name: 'signed_by', args: ['/owner.id'] },
        { sign: '+', name: 'threshold', args: ['2', '/members'] }
      ]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule no_delegated_release { formula { always (not <+RELEASE> (signed_by(/owner.id) or signed_by(/delegate.id))) } }'
    ),
    [
      [{ sign: '-', name: 'RELEASE', args: [] }],
      [
        { sign: '-', name: 'signed_by', args: ['/owner.id'] },
        { sign: '-', name: 'signed_by', args: ['/delegate.id'] }
      ]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule no_quorum_transfer { formula { always (not [+TRANSFER] (signed_by(/owner.id) and threshold(2, /members))) } }'
    ),
    [
      [
        { sign: '+', name: 'TRANSFER', args: [] },
        { sign: '-', name: 'signed_by', args: ['/owner.id'] }
      ],
      [
        { sign: '+', name: 'TRANSFER', args: [] },
        { sign: '-', name: 'threshold', args: ['2', '/members'] }
      ]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule release_after_transfer { formula { always ([+TRANSFER] <+RELEASE> signed_by(/owner.id)) } }'
    ),
    [
      [{ sign: '-', name: 'TRANSFER', args: [] }],
      [
        { sign: '+', name: 'RELEASE', args: [] },
        { sign: '+', name: 'signed_by', args: ['/owner.id'] }
      ]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule transfer_with_release_policy { formula { always (<+TRANSFER> [+RELEASE] signed_by(/owner.id)) } }'
    ),
    [
      [
        { sign: '+', name: 'TRANSFER', args: [] },
        { sign: '-', name: 'RELEASE', args: [] }
      ],
      [
        { sign: '+', name: 'TRANSFER', args: [] },
        { sign: '+', name: 'signed_by', args: ['/owner.id'] }
      ]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule no_release_after_transfer { formula { always (not [+TRANSFER] <+RELEASE> signed_by(/owner.id)) } }'
    ),
    [
      [
        { sign: '+', name: 'TRANSFER', args: [] },
        { sign: '-', name: 'RELEASE', args: [] }
      ],
      [
        { sign: '+', name: 'TRANSFER', args: [] },
        { sign: '-', name: 'signed_by', args: ['/owner.id'] }
      ]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule no_transfer_with_release_policy { formula { always (not <+TRANSFER> [+RELEASE] signed_by(/owner.id)) } }'
    ),
    [
      [{ sign: '-', name: 'TRANSFER', args: [] }],
      [
        { sign: '+', name: 'RELEASE', args: [] },
        { sign: '-', name: 'signed_by', args: ['/owner.id'] }
      ]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule no_admin_or_rule { formula { always (not (adds_rule or signed_by(/admin.id))) } }'
    ),
    [[
      { sign: '-', name: 'adds_rule', args: [] },
      { sign: '-', name: 'signed_by', args: ['/admin.id'] }
    ]]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule no_owner_quorum { formula { always (not (signed_by(/owner.id) and threshold(2, /members))) } }'
    ),
    [
      [{ sign: '-', name: 'signed_by', args: ['/owner.id'] }],
      [{ sign: '-', name: 'threshold', args: ['2', '/members'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule delivery { formula { always (<+oracle_attests(/oracles/delivery.id, "delivered", "true")> true) } }'
    ),
    [
      [{
        sign: '+',
        name: 'oracle_attests',
        args: ['/oracles/delivery.id', '"delivered"', '"true"']
      }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule owner { formula { always ([] signed_by(/owner.id)) } }'
    ),
    [
      [{ sign: '+', name: 'signed_by', args: ['/owner.id'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule owner { formula { always (<> signed_by(/owner.id)) } }'
    ),
    [
      [{ sign: '+', name: 'signed_by', args: ['/owner.id'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule tautology { formula { always (<> true) } }'
    ),
    []
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule impossible { formula { always ([] false) } }'
    ),
    [
      [{ sign: '+', name: '__unsatisfiable_rule__!', args: [] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule impossible { formula { always (<> false) } }'
    ),
    [
      [{ sign: '+', name: '__unsatisfiable_rule__!', args: [] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule tautology { formula { always (not [] false) } }'
    ),
    []
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule tautology { formula { always (not <> false) } }'
    ),
    []
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule can_release { formula { always (can(+RELEASE)) } }'
    ),
    [
      [{ sign: '+', name: 'RELEASE', args: [] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule must_release { formula { always (must(+RELEASE)) } }'
    ),
    [
      [{ sign: '+', name: 'RELEASE', args: [] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule cannot_release { formula { always (not can(+RELEASE)) } }'
    ),
    [
      [{ sign: '-', name: 'RELEASE', args: [] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClausesWithFormulaParser(
      'rule must_not_recv { formula { always (not must(-RECV)) } }'
    ),
    [
      [{ sign: '+', name: 'RECV', args: [] }]
    ]
  );
});

test('rule predicate extraction falls back when formula parser cannot parse documented syntax', () => {
  const validator = new ContractValidator();
  const rule = 'rule membership { formula { always (+modifies(/members) implies +all_signed(/members)) } }';

  assert.equal(
    validator.extractRulePredicateClausesWithFormulaParser(rule),
    null
  );
  assert.deepEqual(
    validator.extractRulePredicateClauses(rule),
    [
      [{ sign: '-', name: 'modifies', args: ['/members'] }],
      [{ sign: '+', name: 'all_signed', args: ['/members'] }]
    ]
  );

  const textualNotRule = 'rule no_rules { formula { always (not adds_rule or signed_by(/admin.id)) } }';
  assert.equal(
    validator.extractRulePredicateClausesWithFormulaParser(textualNotRule),
    null
  );
  assert.deepEqual(
    validator.extractRulePredicateClauses(textualNotRule),
    [
      [{ sign: '-', name: 'adds_rule', args: [] }],
      [{ sign: '+', name: 'signed_by', args: ['/admin.id'] }]
    ]
  );

  const textualMixedRule = 'rule docs { formula { always (signed_by(/a.id) or signed_by(/b.id) and modifies(/docs)) } }';
  assert.equal(
    validator.extractRulePredicateClausesWithFormulaParser(textualMixedRule),
    null
  );
  assert.deepEqual(
    validator.extractRulePredicateClauses(textualMixedRule),
    [
      [{ sign: '+', name: 'signed_by', args: ['/a.id'] }],
      [
        { sign: '+', name: 'signed_by', args: ['/b.id'] },
        { sign: '+', name: 'modifies', args: ['/docs'] }
      ]
    ]
  );

  const eventualRule = 'rule eventual { formula { eventually (signed_by(/owner.id)) } }';
  assert.equal(
    validator.extractRulePredicateClausesWithFormulaParser(eventualRule),
    null
  );
  assert.deepEqual(
    validator.extractRulePredicateClauses(eventualRule),
    [
      [{ sign: '+', name: 'signed_by', args: ['/owner.id'] }]
    ]
  );
});

test('parser-backed nested modal rules constrain model replacements', () => {
  const validator = new ContractValidator();
  const ruleContent = 'rule release_after_transfer { formula { always ([+TRANSFER] <+RELEASE> signed_by(/owner.id)) } }';

  assert.throws(
    () => validator.applyCommit({
      data: {
        method: 'RULE',
        path: '/rules/release-after-transfer-unsafe.modality',
        content: ruleContent,
        model: `
          model release_after_transfer_unsafe_witness {
            initial active
            active -> active [+TRANSFER +RELEASE]
          }
        `
      }
    }),
    /RULE witness model failed: MODEL transition active->active does not satisfy existing rule predicate/
  );

  validator.applyCommit({
    data: {
      method: 'RULE',
      path: '/rules/release-after-transfer.modality',
      content: ruleContent,
      model: `
        model release_after_transfer_witness {
          initial active
          active -> active [-TRANSFER]
          active -> active [+RELEASE +signed_by(/owner.id)]
        }
      `
    }
  });

  assert.doesNotThrow(() => validator.applyCommit({
    data: {
      method: 'MODEL',
      path: '/rules/release-after-transfer-ok.modality',
      content: `
        model release_after_transfer_ok {
          initial active
          active -> active [+TRANSFER +RELEASE +signed_by(/owner.id)]
        }
      `
    }
  }));

  assert.throws(
    () => validator.loadModel('/rules/release-after-transfer-unsafe.modality', `
      model release_after_transfer_unsafe {
        initial active
        active -> active [+TRANSFER +RELEASE]
      }
    `),
    /does not satisfy existing rule predicate -TRANSFER \| \+RELEASE & \+signed_by\(\/owner.id\)/
  );
});

test('parser-backed negated nested modal rules constrain model replacements', () => {
  const validator = new ContractValidator();
  const ruleContent = 'rule no_transfer_with_release_policy { formula { always (not <+TRANSFER> [+RELEASE] signed_by(/owner.id)) } }';

  assert.throws(
    () => validator.applyCommit({
      data: {
        method: 'RULE',
        path: '/rules/no-transfer-with-release-policy-unsafe.modality',
        content: ruleContent,
        model: `
          model no_transfer_with_release_policy_unsafe_witness {
            initial active
            active -> active [+TRANSFER +RELEASE +signed_by(/owner.id)]
          }
        `
      }
    }),
    /RULE witness model failed: MODEL transition active->active does not satisfy existing rule predicate/
  );

  validator.applyCommit({
    data: {
      method: 'RULE',
      path: '/rules/no-transfer-with-release-policy.modality',
      content: ruleContent,
      model: `
        model no_transfer_with_release_policy_witness {
          initial active
          active -> active [-TRANSFER]
          active -> active [+RELEASE -signed_by(/owner.id)]
        }
      `
    }
  });

  assert.doesNotThrow(() => validator.applyCommit({
    data: {
      method: 'MODEL',
      path: '/rules/no-transfer-with-release-policy-ok.modality',
      content: `
        model no_transfer_with_release_policy_ok {
          initial active
          active -> active [+RELEASE -signed_by(/owner.id)]
        }
      `
    }
  }));

  assert.throws(
    () => validator.loadModel('/rules/no-transfer-with-release-policy-unsafe.modality', `
      model no_transfer_with_release_policy_unsafe {
        initial active
        active -> active [+TRANSFER +RELEASE +signed_by(/owner.id)]
      }
    `),
    /does not satisfy existing rule predicate -TRANSFER \| \+RELEASE & -signed_by\(\/owner.id\)/
  );
});

test('parser-backed when rules constrain model replacements', () => {
  const validator = new ContractValidator();
  const ruleContent = 'rule transfer_owner { formula { always (when +TRANSFER also signed_by(/owner.id)) } }';

  assert.throws(
    () => validator.applyCommit({
      data: {
        method: 'RULE',
        path: '/rules/transfer-owner-unsafe.modality',
        content: ruleContent,
        model: `
          model transfer_owner_unsafe_witness {
            initial active
            active -> active [+TRANSFER]
          }
        `
      }
    }),
    /RULE witness model failed: MODEL transition active->active does not satisfy existing rule predicate/
  );

  validator.applyCommit({
    data: {
      method: 'RULE',
      path: '/rules/transfer-owner.modality',
      content: ruleContent,
      model: `
        model transfer_owner_witness {
          initial active
          active -> active [-TRANSFER]
          active -> active [+signed_by(/owner.id)]
        }
      `
    }
  });

  assert.doesNotThrow(() => validator.applyCommit({
    data: {
      method: 'MODEL',
      path: '/rules/transfer-owner-ok.modality',
      content: `
        model transfer_owner_ok {
          initial active
          active -> active [+TRANSFER +signed_by(/owner.id)]
        }
      `
    }
  }));

  assert.throws(
    () => validator.loadModel('/rules/transfer-owner-unsafe.modality', `
      model transfer_owner_unsafe {
        initial active
        active -> active [+TRANSFER]
      }
    `),
    /does not satisfy existing rule predicate -TRANSFER \| \+signed_by\(\/owner.id\)/
  );
});

test('parser-backed when next rules constrain model replacements', () => {
  const validator = new ContractValidator();
  const ruleContent = 'rule next_transfer_owner { formula { always (when +TRANSFER next signed_by(/owner.id)) } }';

  assert.throws(
    () => validator.applyCommit({
      data: {
        method: 'RULE',
        path: '/rules/next-transfer-owner-unsafe.modality',
        content: ruleContent,
        model: `
          model next_transfer_owner_unsafe_witness {
            initial active
            active -> active [+TRANSFER]
          }
        `
      }
    }),
    /RULE witness model failed: MODEL transition active->active does not satisfy existing rule predicate/
  );

  validator.applyCommit({
    data: {
      method: 'RULE',
      path: '/rules/next-transfer-owner.modality',
      content: ruleContent,
      model: `
        model next_transfer_owner_witness {
          initial active
          active -> active [-TRANSFER]
          active -> active [+signed_by(/owner.id)]
        }
      `
    }
  });

  assert.doesNotThrow(() => validator.applyCommit({
    data: {
      method: 'MODEL',
      path: '/rules/next-transfer-owner-ok.modality',
      content: `
        model next_transfer_owner_ok {
          initial active
          active -> active [+TRANSFER +signed_by(/owner.id)]
        }
      `
    }
  }));

  assert.throws(
    () => validator.loadModel('/rules/next-transfer-owner-unsafe.modality', `
      model next_transfer_owner_unsafe {
        initial active
        active -> active [+TRANSFER]
      }
    `),
    /does not satisfy existing rule predicate -TRANSFER \| \+signed_by\(\/owner.id\)/
  );
});

test('rule predicate extraction flips explicit negation polarity', () => {
  const validator = new ContractValidator();

  validator.applyCommit({
    data: {
      method: 'RULE',
      path: '/rules/membership.modality',
      content: 'rule membership { formula { always (!+modifies(/members) | +all_signed(/members)) } }',
      model: `
        model membership_witness {
          initial active
          active -> active [-modifies(/members)]
          active -> active [+all_signed(/members)]
        }
      `
    }
  });

  assert.doesNotThrow(() => validator.applyCommit({
    data: {
      method: 'MODEL',
      path: '/rules/membership-ok.modality',
      content: `
        model membership_ok {
          initial active
          active -> active [-modifies(/members)]
          active -> active [+all_signed(/members)]
        }
      `
    }
  }));

  assert.throws(
    () => validator.loadModel('/rules/membership-unsafe.modality', `
      model membership_unsafe {
        initial active
        active -> active [+modifies(/members)]
      }
    `),
    /does not satisfy existing rule predicate -modifies\(\/members\) \| \+all_signed\(\/members\)/
  );
});

test('nested rule disjunctions keep surrounding conjunctions', () => {
  const validator = new ContractValidator();

  validator.applyCommit({
    data: {
      method: 'RULE',
      path: '/rules/docs.modality',
      content: 'rule docs { formula { always ((+signed_by(/members/alice.id) | +signed_by(/members/bob.id)) & +modifies(/docs)) } }',
      model: `
        model docs_witness {
          initial active
          active -> active [+signed_by(/members/alice.id) +modifies(/docs)]
          active -> active [+signed_by(/members/bob.id) +modifies(/docs)]
        }
      `
    }
  });

  assert.doesNotThrow(() => validator.applyCommit({
    data: {
      method: 'MODEL',
      path: '/rules/docs-ok.modality',
      content: `
        model docs_ok {
          initial active
          active -> active [+signed_by(/members/alice.id) +modifies(/docs)]
          active -> active [+signed_by(/members/bob.id) +modifies(/docs)]
        }
      `
    }
  }));

  assert.throws(
    () => validator.loadModel('/rules/docs-unsafe.modality', `
      model docs_unsafe {
        initial active
        active -> active [+signed_by(/members/alice.id)]
      }
    `),
    /does not satisfy existing rule predicate/
  );
});

test('compound rule predicate negation applies De Morgan clauses', () => {
  const validator = new ContractValidator();

  assert.deepEqual(
    validator.extractRulePredicateClauses('rule no_pair { formula { always (!(+signed_by(/members/alice.id) | +signed_by(/members/bob.id))) } }'),
    [[
      { sign: '-', name: 'signed_by', args: ['/members/alice.id'] },
      { sign: '-', name: 'signed_by', args: ['/members/bob.id'] }
    ]]
  );
});

test('rule predicate extraction supports textual implication', () => {
  const validator = new ContractValidator();

  assert.deepEqual(
    validator.extractRulePredicateClauses('rule membership { formula { always (+modifies(/members) implies +all_signed(/members)) } }'),
    [
      [{ sign: '-', name: 'modifies', args: ['/members'] }],
      [{ sign: '+', name: 'all_signed', args: ['/members'] }]
    ]
  );

  validator.applyCommit({
    data: {
      method: 'RULE',
      path: '/rules/membership.modality',
      content: 'rule membership { formula { always (+modifies(/members) implies +all_signed(/members)) } }',
      model: `
        model membership_witness {
          initial active
          active -> active [-modifies(/members)]
          active -> active [+all_signed(/members)]
        }
      `
    }
  });

  assert.throws(
    () => validator.loadModel('/rules/membership-unsafe.modality', `
      model membership_unsafe {
        initial active
        active -> active [+modifies(/members)]
      }
    `),
    /does not satisfy existing rule predicate -modifies\(\/members\) \| \+all_signed\(\/members\)/
  );
});

test('rule predicate extraction treats bare predicate calls as positive predicates', () => {
  const validator = new ContractValidator();

  assert.deepEqual(
    validator.extractRulePredicateClauses('rule membership { formula { always (modifies(/members) implies all_signed(/members)) } }'),
    [
      [{ sign: '-', name: 'modifies', args: ['/members'] }],
      [{ sign: '+', name: 'all_signed', args: ['/members'] }]
    ]
  );

  assert.deepEqual(
    validator.extractRulePredicateClauses('rule docs { formula { always (signed_by(/members/alice.id) or signed_by(/members/bob.id)) } }'),
    [
      [{ sign: '+', name: 'signed_by', args: ['/members/alice.id'] }],
      [{ sign: '+', name: 'signed_by', args: ['/members/bob.id'] }]
    ]
  );
});

test('rule predicate extraction supports modal action implications', () => {
  const validator = new ContractValidator();

  assert.deepEqual(
    validator.extractRulePredicateClauses('rule owner_transfer { formula { always ([+TRANSFER] implies signed_by(/owner.id)) } }'),
    [
      [{ sign: '-', name: 'TRANSFER', args: [] }],
      [{ sign: '+', name: 'signed_by', args: ['/owner.id'] }]
    ]
  );
});

test('rule predicate extraction supports bare adds_rule predicates', () => {
  const validator = new ContractValidator();

  assert.deepEqual(
    validator.extractRulePredicateClauses('rule no_rules { formula { always (!adds_rule | signed_by(/admin.id)) } }'),
    [
      [{ sign: '-', name: 'adds_rule', args: [] }],
      [{ sign: '+', name: 'signed_by', args: ['/admin.id'] }]
    ]
  );
});

test('rule predicate extraction supports textual not', () => {
  const validator = new ContractValidator();

  assert.deepEqual(
    validator.extractRulePredicateClauses('rule history { formula { always (not modifies(/history)) } }'),
    [[{ sign: '-', name: 'modifies', args: ['/history'] }]]
  );
});

test('rule predicate extraction supports arrow implications', () => {
  const validator = new ContractValidator();

  assert.deepEqual(
    validator.extractRulePredicateClauses('rule expiry { formula { always (after(/deadlines/expiry.datetime) -> signed_by(/users/buyer.id)) } }'),
    [
      [{ sign: '-', name: 'after', args: ['/deadlines/expiry.datetime'] }],
      [{ sign: '+', name: 'signed_by', args: ['/users/buyer.id'] }]
    ]
  );
});

test('rule predicate extraction supports modal multi-argument predicates', () => {
  const validator = new ContractValidator();

  assert.deepEqual(
    validator.extractRulePredicateClauses('rule delivery { formula { always ([+RELEASE] implies <+oracle_attests(/oracles/delivery.id, "delivered", "true")> true) } }'),
    [
      [{ sign: '-', name: 'RELEASE', args: [] }],
      [{ sign: '+', name: 'oracle_attests', args: ['/oracles/delivery.id', '"delivered"', '"true"'] }]
    ]
  );
});

test('parser-backed threshold rules constrain model replacements', () => {
  const validator = new ContractValidator();
  const ruleContent = 'rule quorum { formula { always (threshold(2, /members)) } }';

  assert.throws(
    () => validator.applyCommit({
      data: {
        method: 'RULE',
        path: '/rules/quorum-unsafe.modality',
        content: ruleContent,
        model: `
          model quorum_unsafe_witness {
            initial active
            active -> active [+any_signed(/members)]
          }
        `
      }
    }),
    /RULE witness model failed: MODEL transition active->active does not satisfy existing rule predicate/
  );

  validator.applyCommit({
    data: {
      method: 'RULE',
      path: '/rules/quorum.modality',
      content: ruleContent,
      model: `
        model quorum_witness {
          initial active
          active -> active [+threshold(2, /members)]
        }
      `
    }
  });

  assert.doesNotThrow(() => validator.applyCommit({
    data: {
      method: 'MODEL',
      path: '/rules/quorum-ok.modality',
      content: `
        model quorum_ok {
          initial active
          active -> active [+threshold(2, /members)]
        }
      `
    }
  }));

  assert.throws(
    () => validator.loadModel('/rules/quorum-unsafe.modality', `
      model quorum_unsafe {
        initial active
        active -> active [+any_signed(/members)]
      }
    `),
    /does not satisfy existing rule predicate \+threshold\(2, \/members\)/
  );
});

test('parser-backed oracle diamond rules constrain model replacements', () => {
  const validator = new ContractValidator();
  const ruleContent = 'rule delivery { formula { always (<+oracle_attests(/oracles/delivery.id, "delivered", "true")> true) } }';

  assert.throws(
    () => validator.applyCommit({
      data: {
        method: 'RULE',
        path: '/rules/delivery-unsafe.modality',
        content: ruleContent,
        model: `
          model delivery_unsafe_witness {
            initial active
            active -> active [+signed_by(/owner.id)]
          }
        `
      }
    }),
    /RULE witness model failed: MODEL transition active->active does not satisfy existing rule predicate/
  );

  validator.applyCommit({
    data: {
      method: 'RULE',
      path: '/rules/delivery.modality',
      content: ruleContent,
      model: `
        model delivery_witness {
          initial active
          active -> active [+oracle_attests(/oracles/delivery.id, "delivered", "true")]
        }
      `
    }
  });

  assert.doesNotThrow(() => validator.applyCommit({
    data: {
      method: 'MODEL',
      path: '/rules/delivery-ok.modality',
      content: `
        model delivery_ok {
          initial active
          active -> active [+oracle_attests(/oracles/delivery.id, "delivered", "true")]
        }
      `
    }
  }));

  assert.throws(
    () => validator.loadModel('/rules/delivery-unsafe.modality', `
      model delivery_unsafe {
        initial active
        active -> active [+signed_by(/owner.id)]
      }
    `),
    /does not satisfy existing rule predicate \+oracle_attests\(\/oracles\/delivery.id, "delivered", "true"\)/
  );
});

test('parser-backed can macro rules constrain model replacements', () => {
  const validator = new ContractValidator();
  const ruleContent = 'rule releasable { formula { always (can(+RELEASE)) } }';

  assert.throws(
    () => validator.applyCommit({
      data: {
        method: 'RULE',
        path: '/rules/releasable-unsafe.modality',
        content: ruleContent,
        model: `
          model releasable_unsafe_witness {
            initial active
            active -> active [+signed_by(/owner.id)]
          }
        `
      }
    }),
    /RULE witness model failed: MODEL transition active->active does not satisfy existing rule predicate/
  );

  validator.applyCommit({
    data: {
      method: 'RULE',
      path: '/rules/releasable.modality',
      content: ruleContent,
      model: `
        model releasable_witness {
          initial active
          active -> active [+RELEASE]
        }
      `
    }
  });

  assert.doesNotThrow(() => validator.applyCommit({
    data: {
      method: 'MODEL',
      path: '/rules/releasable-ok.modality',
      content: `
        model releasable_ok {
          initial active
          active -> active [+RELEASE]
        }
      `
    }
  }));

  assert.throws(
    () => validator.loadModel('/rules/releasable-unsafe.modality', `
      model releasable_unsafe {
        initial active
        active -> active [+signed_by(/owner.id)]
      }
    `),
    /does not satisfy existing rule predicate \+RELEASE/
  );
});

test('parser-backed must macro rules constrain model replacements', () => {
  const validator = new ContractValidator();
  const ruleContent = 'rule required_release { formula { always (must(+RELEASE)) } }';

  assert.throws(
    () => validator.applyCommit({
      data: {
        method: 'RULE',
        path: '/rules/required-release-unsafe.modality',
        content: ruleContent,
        model: `
          model required_release_unsafe_witness {
            initial active
            active -> active [+signed_by(/owner.id)]
          }
        `
      }
    }),
    /RULE witness model failed: MODEL transition active->active does not satisfy existing rule predicate/
  );

  validator.applyCommit({
    data: {
      method: 'RULE',
      path: '/rules/required-release.modality',
      content: ruleContent,
      model: `
        model required_release_witness {
          initial active
          active -> active [+RELEASE]
        }
      `
    }
  });

  assert.doesNotThrow(() => validator.applyCommit({
    data: {
      method: 'MODEL',
      path: '/rules/required-release-ok.modality',
      content: `
        model required_release_ok {
          initial active
          active -> active [+RELEASE]
        }
      `
    }
  }));

  assert.throws(
    () => validator.loadModel('/rules/required-release-unsafe.modality', `
      model required_release_unsafe {
        initial active
        active -> active [+signed_by(/owner.id)]
      }
    `),
    /does not satisfy existing rule predicate \+RELEASE/
  );
});

test('threshold predicates require enough distinct member signatures', () => {
  const validator = new ContractValidator();

  for (const [name, key] of [
    ['alice', 'alice-key'],
    ['bob', 'bob-key'],
    ['carol', 'carol-key']
  ]) {
    validator.applyCommit({
      data: {
        method: 'POST',
        path: `/members/${name}.id`,
        content: key
      }
    });
  }

  validator.applyCommit({
    data: {
      method: 'MODEL',
      path: '/rules/threshold.modality',
      content: `
        model treasury {
          initial active
          active -> active [+threshold(2, /members)]
        }
      `
    }
  });

  assert.equal(validator.validateCommit({
    data: {
      method: 'POST',
      path: '/payments/invoice.json',
      content: { amount: 100 },
      signatures: [
        { signer_key: 'alice-key' },
        { signer_key: 'bob-key' }
      ]
    }
  }).ok, true);

  assert.equal(validator.validateCommit({
    data: {
      method: 'POST',
      path: '/payments/invoice.json',
      content: { amount: 100 },
      signatures: [
        { signer_key: 'alice-key' },
        { signer_key: 'outsider-key' }
      ]
    }
  }).ok, false);
});

test('model state tracks nondeterministic branches as a set', () => {
  const validator = new ContractValidator();

  validator.applyCommit({
    data: {
      method: 'POST',
      path: '/members/alice.id',
      content: 'alice-key'
    }
  });
  validator.applyCommit({
    data: {
      method: 'POST',
      path: '/members/bob.id',
      content: 'bob-key'
    }
  });

  validator.applyCommit({
    data: {
      method: 'MODEL',
      path: '/rules/branches.modality',
      content: `
        model branches {
          initial active
          active -> reviewing [+signed_by(/members/alice.id)]
          active -> approved [+signed_by(/members/alice.id)]
          approved -> done [+signed_by(/members/bob.id)]
        }
      `
    }
  });

  validator.applyCommit({
    data: {
      method: 'POST',
      path: '/requests/1.json',
      content: { requested: true },
      signatures: [{ signer_key: 'alice-key' }]
    }
  });

  assert.deepEqual(new Set(validator.getState().currentStates), new Set(['reviewing', 'approved']));

  assert.equal(validator.validateCommit({
    data: {
      method: 'POST',
      path: '/requests/1.json',
      content: { approved: true },
      signatures: [{ signer_key: 'bob-key' }]
    }
  }).ok, true);

  validator.applyCommit({
    data: {
      method: 'POST',
      path: '/requests/1.json',
      content: { approved: true },
      signatures: [{ signer_key: 'bob-key' }]
    }
  });

  assert.deepEqual(validator.getState().currentStates, ['done']);
});

test('MODEL replacement is validated by the current model before taking effect', () => {
  const validator = new ContractValidator();

  validator.applyCommit({
    data: {
      method: 'POST',
      path: '/members/alice.id',
      content: 'alice-key'
    }
  });
  validator.applyCommit({
    data: {
      method: 'POST',
      path: '/members/bob.id',
      content: 'bob-key'
    }
  });

  validator.applyCommit({
    data: {
      method: 'MODEL',
      path: '/rules/alice.modality',
      content: `
        model alice_only {
          initial active
          active -> active [+signed_by(/members/alice.id)]
        }
      `
    }
  });

  assert.throws(
    () => validator.applyCommit({
      data: {
        method: 'MODEL',
        path: '/rules/bob.modality',
        content: `
          model bob_only {
            initial active
            active -> active [+signed_by(/members/bob.id)]
          }
        `,
        signatures: [{ signer_key: 'bob-key' }]
      }
    }),
    /MODEL is not allowed/
  );

  validator.applyCommit({
    data: {
      method: 'MODEL',
      path: '/rules/bob.modality',
      content: `
        model bob_only {
          initial active
          active -> active [+signed_by(/members/bob.id)]
        }
      `,
      signatures: [{ signer_key: 'alice-key' }]
    }
  });

  assert.equal(validator.validateCommit({
    data: {
      method: 'POST',
      path: '/docs/bob.md',
      content: 'bob can now write',
      signatures: [{ signer_key: 'bob-key' }]
    }
  }).ok, true);

  assert.equal(validator.validateCommit({
    data: {
      method: 'POST',
      path: '/docs/alice.md',
      content: 'alice no longer can write',
      signatures: [{ signer_key: 'alice-key' }]
    }
  }).ok, false);
});

test('validateContractLogic replays MODEL replacement within a batch', async () => {
  const store = {
    pullCommits() {
      return [];
    }
  };
  const baseCommits = [
    {
      data: {
        method: 'POST',
        path: '/members/alice.id',
        content: 'alice-key'
      }
    },
    {
      data: {
        method: 'POST',
        path: '/members/bob.id',
        content: 'bob-key'
      }
    },
    {
      data: {
        method: 'MODEL',
        path: '/rules/alice.modality',
        content: `
          model alice_only {
            initial active
            active -> active [+signed_by(/members/alice.id)]
          }
        `
      }
    },
    {
      data: {
        method: 'POST',
        path: '/docs/alice.md',
        content: 'alice write',
        signatures: [{ signer_key: 'alice-key' }]
      }
    }
  ];
  const bobModel = {
    data: {
      method: 'MODEL',
      path: '/rules/bob.modality',
      content: `
        model bob_only {
          initial active
          active -> active [+signed_by(/members/bob.id)]
        }
      `,
      signatures: [{ signer_key: 'alice-key' }]
    }
  };
  const bobWrite = {
    data: {
      method: 'POST',
      path: '/docs/bob.md',
      content: 'bob write',
      signatures: [{ signer_key: 'bob-key' }]
    }
  };

  const valid = await validateContractLogic(store, 'contract', [
    ...baseCommits,
    bobModel,
    bobWrite
  ]);

  assert.equal(valid.valid, true);
  assert.equal(valid.state.model.name, 'bob_only');

  const invalid = await validateContractLogic(store, 'contract', [
    ...baseCommits,
    {
      data: {
        ...bobModel.data,
        signatures: [{ signer_key: 'bob-key' }]
      }
    },
    bobWrite
  ]);

  assert.equal(invalid.valid, false);
  assert.match(invalid.errors[0], /MODEL is not allowed/);
});

test('validateContractLogic anchors rules to later MODEL replacements', async () => {
  const store = {
    pullCommits() {
      return [];
    }
  };
  const baseCommits = [
    {
      data: {
        method: 'POST',
        path: '/members/alice.id',
        content: 'alice-key'
      }
    },
    {
      data: {
        method: 'MODEL',
        path: '/rules/open.modality',
        content: `
          model open {
            initial active
            active -> active []
          }
        `
      }
    },
    {
      data: {
        method: 'RULE',
        path: '/rules/signed.modality',
        content: 'rule signed { formula { always (+any_signed(/members)) } }',
        model: `
          model signed_witness {
            initial active
            active -> active [+any_signed(/members)]
          }
        `
      }
    },
    {
      data: {
        method: 'POST',
        path: '/docs/unsigned.md',
        content: 'rules do not validate commits directly'
      }
    }
  ];

  const unsignedPost = await validateContractLogic(store, 'contract', baseCommits);
  assert.equal(unsignedPost.valid, true);

  const invalidReplacement = await validateContractLogic(store, 'contract', [
    ...baseCommits,
    {
      data: {
        method: 'MODEL',
        path: '/rules/open-again.modality',
        content: `
          model open_again {
            initial active
            active -> active []
          }
        `
      }
    }
  ]);

  assert.equal(invalidReplacement.valid, false);
  assert.match(invalidReplacement.errors[0], /does not satisfy existing rule predicate/);

  const validReplacement = await validateContractLogic(store, 'contract', [
    ...baseCommits,
    {
      data: {
        method: 'MODEL',
        path: '/rules/signed-model.modality',
        content: `
          model signed_model {
            initial active
            active -> active [+any_signed(/members)]
          }
        `
      }
    }
  ]);

  assert.equal(validReplacement.valid, true);
  assert.equal(validReplacement.state.model.name, 'signed_model');
});

test('RULE commits require a satisfying witness model', () => {
  const validator = new ContractValidator();

  assert.throws(
    () => validator.applyCommit({
      data: {
        method: 'RULE',
        path: '/rules/signed.modality',
        content: 'rule signed { formula { always (+any_signed(/members)) } }'
      }
    }),
    /RULE requires a witness model/
  );

  assert.throws(
    () => validator.applyCommit({
      data: {
        method: 'RULE',
        path: '/rules/signed.modality',
        content: 'rule signed { formula { always (+any_signed(/members)) } }',
        model: `
          model open {
            initial active
            active -> active []
          }
        `
      }
    }),
    /RULE witness model failed/
  );

  assert.throws(
    () => validator.applyCommit({
      data: {
        method: 'RULE',
        path: '/rules/impossible.modality',
        content: 'rule impossible { formula { always (false) } }',
        model: `
          model impossible {
            initial active
            active -> active []
          }
        `
      }
    }),
    /RULE witness model failed/
  );
});

test('existing RULE history replays without witness while new RULE commits require one', async () => {
  const legacyRule = {
    data: {
      method: 'RULE',
      path: '/rules/signed.modality',
      content: 'rule signed { formula { always (+any_signed(/members)) } }'
    }
  };
  const validator = new ContractValidator();

  assert.doesNotThrow(() => validator.loadFromCommits([legacyRule]));

  const store = {
    pullCommits() {
      return [legacyRule];
    }
  };

  const replacement = await validateContractLogic(store, 'contract', [
    {
      data: {
        method: 'MODEL',
        path: '/rules/open.modality',
        content: `
          model open {
            initial active
            active -> active []
          }
        `
      }
    }
  ]);

  assert.equal(replacement.valid, false);
  assert.match(replacement.errors[0], /does not satisfy existing rule predicate/);

  const newRule = await validateContractLogic({ pullCommits: () => [] }, 'contract', [legacyRule]);
  assert.equal(newRule.valid, false);
  assert.match(newRule.errors[0], /RULE requires a witness model/);
});
