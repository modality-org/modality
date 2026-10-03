---
sidebar_position: 3
title: Hash Commitments
---

# Hash Commitments: Anchor Now, Reveal Later

A network with a **hash lane** orders a commit's **hash** without its body.
You can fix a commit's place in the log first and send its body later. The
body is checked when it arrives, like any other commit.

A hash commitment is not an accepted commit. Nothing is checked against the
contract's rules until the body is revealed, and nothing can read a hash as
contract state.

## Two lanes

| | Hash lane | Pushed commits |
|---|---|---|
| What is sent | Contract id, commit hash, parent hash, a signature, and a hashtax proof | The commit body |
| What a sequencer checks before voting | Record shape and size, the signature, the hashtax, the epoch anchor, and the contract's signer set if it has one | Signatures on the sequencer block |
| What is checked when the block is applied | Nothing beyond the record: it is indexed | The contract's model and rules |
| What nodes keep | The hash record | The commit, and the state it changes |
| What it proves | This hash was ordered at this point | The commit met the contract's rules |
| What can use it | A later reveal of the same body | Pull, replay, REPOST, RECV, programs |

Each lane has its own limit. Hash commitments cannot use up room meant for
pushed commits.

## A hash commitment

One record per commit:

- **contract id** and **commit hash**: the same ids a pushed commit uses, so
  a later body either matches or does not
- **parent hash**, omitted for the genesis commit
- the **signer's Modality ID** and an **ed25519 signature** over the record
- a **hashtax proof**: an anchor and a nonce
- on the genesis commit only, an optional **signer set**, signed by the
  contract's own key

It carries no body, state, model, or rules. A node checks a record without
looking anything up about the contract, except its signer set.

## Hashtax

A hash commitment costs **work**, not coin. The proof is a nonce such that

```
sha256(record, signature, epoch anchor, nonce)
```

has at least the network's `floor_bits` leading zero bits. It takes one
SHA-256 to check, which is cheap enough for every sequencer to check every
record before voting. It is not the miner puzzle.

The signature is inside the hash, so a proof cannot be moved onto another
commit or signer.

**Epoch anchor.** The proof names a miner block: the last block of epoch
`E − 2`, which the committee for epoch `E` was nominated from. Every honest
sequencer already agrees on that block. A sequencer accepts anchors for its
own epoch and the epochs on either side, so sequencers an epoch apart still
agree. An older proof, or one bound to a private chain, is refused. A
network without a miner chain uses one fixed anchor derived from its name.

## Quota

A sequencer block carries at most `quota_per_block` hash commitments. When
more are waiting, the proposer takes the ones with the **most work** (lowest
digest), tie-broken by commit hash. Up to four blocks' worth of the rest wait
for later rounds, and anything beyond that is dropped. A sequencer does not
vote for a block that carries more than the quota, a record that repeats, or
a record that fails any check. More work is how you bid for a place when the
lane is full.

## Signer set

By default anyone may anchor hashes for a contract. To keep other keys off
it, create the contract with its signers:

```bash
modal contract create --signer alice.mod_passfile --signer bob.mod_passfile
```

The set is fixed at creation, in two places:

- **In the genesis body.** The genesis commit posts the keys at
  `/signers/1.id`, `/signers/2.id`, …, a model whose every step after
  genesis is signed by one of them, and the creation rule
  `always([-signed_by(/signers/1.id) -signed_by(/signers/2.id)] false)`.
- **For the hash lane.** A contract id is the public key `create` makes, in the Modality spelling.
  That key signs the set once, and is then discarded. The signature is kept
  in `.contract/signer_set.json`, and `anchor` posts it with the set on the
  genesis hash commitment.

The genesis record must be signed by a key in the set, and the set must
carry the contract key's signature. Anyone who sees the genesis hash can
anchor it, but only the contract's creator can post its set. Once the set
is certified, a sequencer does not include a record for the contract signed
by any other key. It binds from the round after the one that certified it.
The set cannot be replaced or widened later.

When several blocks carry a record for the same commit, or a set for the
same contract, every node keeps the one in the lowest round. Between blocks
of one round it keeps the lowest sequencer id, then the lowest work digest.
Nodes that apply the same blocks in a different order keep the same records.

The set limits who can extend the contract on the hash lane. Who can extend
it with bodies is up to the contract's rules, and the creation rule in the
genesis body says the same thing.

## Reveal

A reveal is a normal push with `--reveal`. For each commit, the node checks
that:

1. the body hashes to the commit id it is sent under
2. a certified hash commitment names that commit
3. the commit extends the contract's current head, as every pushed commit
   must: bodies are revealed in order, from genesis
4. for a genesis body, when the contract has a certified signer set: the
   body posts exactly those keys under its creation rule
5. the contract's model and rules accept it

A reveal that fails the first check is refused. One that fails the second is
refused when pushed; a node that applies the block before it has indexed
the hash commitment holds the reveal until it has. A commit refused by the
rules leaves no state. Its hash stays in the log as an anchor to bytes that
were refused.

A commit pushed without `--reveal` does not need a hash commitment. That
path is unchanged.

## What stays true

- An accepted commit always had its body in the log and was checked against
  the contract's rules. A hash commitment is neither.
- Pull and replay return pushed commits only. A hash-only commit has no body
  to return.
- REPOST and RECV name sequenced commits. A REPOST whose source commit was
  only anchored is refused.
- Sequencers never run the contract's rules to vote on a hash commitment.

## Try it

```bash
# Anchor every commit not yet pushed; print what was queued
modal contract anchor --remote <node multiaddr>

# Ask the node what it has for a commit: unknown, anchored, or revealed
modal contract anchor --status --commit <commit id> --remote <node multiaddr>

# A contract only Alice can extend; its genesis record posts the set
modal contract create --signer alice.mod_passfile
modal contract anchor --sign alice.mod_passfile

# Later: send the bodies
modal contract push --reveal
```

See [Contract Commands](../cli/contract-commands.md#anchor) for every option.

## Network parameters

A network turns the hash lane on with `hash_lane` in its network config:

```json
"hash_lane": {
  "quota_per_block": 64,
  "floor_bits": 16,
  "algorithm": "sha256",
  "max_signers": 16
}
```

A network without `hash_lane` has no hash lane, and its sequencers refuse to
vote for a block that carries a hash commitment. `floor_bits` is fixed for
now: it does not yet retarget with how full the lane is.
