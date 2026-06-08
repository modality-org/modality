import test from 'node:test';
import assert from 'node:assert/strict';

import { ContractValidator } from './contract-validator.js';

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
      content: 'rule signed { formula { always (+any_signed(/members)) } }'
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
