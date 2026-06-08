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
