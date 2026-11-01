# Alice and Bob: rules, models and signatures

Two parties start a contract with no rules and agree on two. Each brings a
model that satisfies those rules but serves its author. A stranger and a
forger try their luck along the way. The whole story is in
[AliceBob.lean](../../AliceBob.lean), about 250 lines.

```sh
lake exe alice-bob
```

It runs in well under a second. Every quoted line below is from that output.
The key fingerprints (`d8d5c4bc…`) are the same on every run, because the
demo keys are derived from fixed phrases.

Signatures here are ed25519. `lake exe alice-bob --crypto oracle` tells the
same story with the fast oracle scheme. Only the key fingerprints and
commit ids differ.

## The pieces

Before the story, the parts it is built from:

- **State.** The contract state is an ordinary Lean structure,
  [`Chat`](../../AliceBob.lean#L31). It holds the messages, how often each
  party has spoken, and which key belongs to which name. Each accepted
  commit's actions are folded into it by [`apply`](../../AliceBob.lean#L38).
- **Commits.** A commit is a list of actions (`say who text`,
  `bindId name key`), any rules or model it adds, and ed25519 signatures.
  Each signature covers the contract id, the parent commit, and everything
  in the commit.
- **Predicates.** These are Lean functions that look at a commit and answer
  true, false or unknown.
  - `signed_by(alice)` looks up the key bound to `alice` in the state before
    the commit, and checks whether that key signed.
  - `signed_by_all(parties)` and `adds_rule` are also built in.
  - [`shouting`](../../AliceBob.lean#L56) and
    [`chatty`](../../AliceBob.lean#L68) are custom predicates written for
    this demo.
- **Rules.** These are LTL formulas over predicates, such as
  [`either_signs`](../../AliceBob.lean#L78):
  `always(signed_by(alice) | signed_by(bob))`. A rule binds every commit
  after the one that adds it.
- **Models.** A model is a labeled transition system. Each edge carries a
  label like `-adds_rule +signed_by(alice)`, and a commit may take that edge
  only if its predicates give exactly those values. Every new commit must
  take some edge of the model in force. Every new model must replay the
  whole history and satisfy every rule.

## 1. No rules yet

```
== No rules yet: the open model takes any commit
  #0 accepted   alice and bob bind their names to their keys  (model at [q0])
  alice = d8d5c4bc…  bob = 0da0c84a…
  #1 accepted   alice says hello  (model at [q0])
  #2 accepted   carol, a stranger, says hey  (model at [q0])
```

A new contract has the open model: one state `q0` with an unlabeled loop, so
any validly signed commit is accepted.

The first commit binds `alice` and `bob` to their public keys, and both of
them sign it. From then on, `signed_by(alice)` means "signed by key
`d8d5c4bc…`". Carol has a key but no name in this contract. Her commit still
goes through, because nothing forbids it yet.

## 2. Mallory tries to sign as Alice

```
== Mallory, with no key of theirs, tries to sign as Alice
     refused    a commit carrying Alice's key and Mallory's signature
               the signature for key d8d5c4bc… does not verify over this commit
     refused    Alice's signature on 'hello', copied onto a new message
               the signature for key d8d5c4bc… does not verify over this commit
     refused    'hello' itself, replayed at the head
               the signature for key d8d5c4bc… does not verify over this commit
```

All three attempts are refused before any rule is checked: the contract
verifies every signature first.

1. **Wrong key.** Mallory signs with their own key and claims the signature
   is Alice's. It does not verify against Alice's key.
2. **Copied signature.** Alice's real signature on "hello" covers that
   commit's actions and message, not Mallory's new message.
3. **Replayed commit.** Even the "hello" commit, resent unchanged, fails. Its
   signature named the parent it was signed against, and the head has moved
   on since.

This mirrors Modality's `signing_payload`: a signature holds for one
contract at one point in its log.

## 3. Alice and Bob agree on rules; Alice brings a model

```
  either_signs: always((signed_by(alice) | signed_by(bob)))
  both_for_rules: always((adds_rule -> signed_by_all(parties)))
  Alice's model:
model Contract {
  part flow {
    q0 --> q0: -adds_rule
    q0 --> q1: +adds_rule
    q1 --> q1: -adds_rule +signed_by(alice)
    q1 --> q1: +adds_rule +signed_by_all(parties)
  }
}
```

[Alice's model](../../AliceBob.lean#L91) has two states:

- **`q0` is the past.** Any commit that doesn't add a rule loops there, so
  the model can replay the earlier history.
- **The edge to `q1`** is taken by this commit, the one that adds the rules.
- **`q1` is the future.** It has one edge for ordinary commits, which
  requires Alice's signature. It has one edge for rule changes, which
  requires both signatures.

Every path from `q1` satisfies both rules. Alice's model is still lopsided:
Bob can never sign an ordinary commit on his own.

```
  without the predicate theory: refused, rule either_signs violated:
      then forever:
        q1 --> q1 [+adds_rule +signed_by_all(parties)] where -signed_by(alice), -signed_by(bob)
  #3 accepted   rules and Alice's model, signed by both  (model at [q1])
```

Before committing, the demo checks Alice's model **without the predicate
theory**, and the checker refuses it with a counterexample. You read it as
"loop on this edge forever, with `signed_by(alice)` and `signed_by(bob)`
both false". A predicate is a black box, so without help the checker
assumes `signed_by_all(parties)` can be true while `signed_by(alice)` is
false.

The theory supplies the help. [`groupFacts`](../../Vibecheck/Theory.lean)
states that `signed_by_all(parties)` implies `signed_by(alice)`, and the
same for `bob`. Each of those facts comes with a Lean proof. With the
theory, the counterexample is impossible, and commit `#3` is accepted.

## 4. Under Alice's model

```
  #4 accepted   alice says the rules are in  (model at [q1])
  (the rules alone allow bob's next commit: true; the model decides)
     refused    bob says my turn
               no transition from [q1] matches the commit (nearest edge wants +signed_by(alice), got false)
     refused    bob posts his model, signed by bob
               no transition from [q1] matches the commit (nearest edge wants +signed_by(alice), got false)
```

This is the point of the scene. Bob's message satisfies both rules: it is
signed by one of the parties and adds no rule. It is still refused, because
**commits are checked against the model, not the rules**. The refusal names
the nearest edge and what it wanted.

Bob can't replace the model on his own either. That commit is an ordinary
commit like any other, so it needs Alice's signature too.

## 5. Bob drafts a model and checks it first

```
  careless draft: rule either_signs violated:
      then forever:
        q1 --> q1 [-adds_rule] where -signed_by(alice), -signed_by(bob)
  Bob's model: holds
```

[`Contract.tryModel`](../../Vibecheck/Contract.lean) lets a party test a
model before anyone signs it. Bob's
[careless draft](../../AliceBob.lean#L106) drops the signature requirement
on ordinary commits. The checker finds the unsigned loop that breaks
`either_signs`.

His [real model](../../AliceBob.lean#L98) has one ordinary edge for each
party, and it passes. Alice agrees to it, and it goes in as commit `#5`,
signed by both:

```
  #5 accepted   bob's model, co-signed by alice  (model at [q1])
```

## 6. Under Bob's model

```
  #6 accepted   bob says my turn  (model at [q1])
     refused    carol says hey
               no transition from [q1] matches the commit (nearest edge wants +signed_by(alice), got false)
  #7 accepted   bob binds the name alice to his own key  (model at [q1])
  alice is still d8d5c4bc…: a name binds once
     refused    bob adds no_shouting alone
               no transition from [q1] matches the commit (nearest edge wants -adds_rule, got true)
```

- **Bob can speak now.** His own signature is enough for an ordinary
  commit.
- **Carol is out.** Her key isn't bound to either party's name, so the
  `either_signs` rule now excludes her.
- **Bob tries to rebind `alice`.** The commit is accepted, since Bob may sign
  an ordinary commit. But this state binds a name only once (see
  [`apply`](../../AliceBob.lean#L38)), so `alice` keeps her key. If names
  could be rebound, a contract would need a rule against it, as the chess
  contract's `roster_locked` is.
- **Bob can't add a rule alone.** A commit that adds a rule must take the
  `+adds_rule` edge, which requires `signed_by_all(parties)`.

## 7. A rule on a custom predicate, with a synthesized model

```
  no_shouting: always(!shouting)
  synthesized:
model Contract {
  part flow {
    q0 --> q0: -adds_rule
    q0 --> q1: +adds_rule
    q1 --> q2: +signed_by(bob) +signed_by_all(parties) +adds_rule
    q1 --> q1: +signed_by(bob) -adds_rule
    ...
    q2 --> q2: +signed_by(alice) -adds_rule -shouting
  }
}
  #8 accepted   no_shouting with the synthesized model, signed by both  (model at [q2])
```

[`shouting`](../../AliceBob.lean#L56) is a custom predicate: true when a
message is all capitals. Nobody writes this model by hand.
[`Synth.forContract`](../../Vibecheck/Synth.lean) builds it in epochs:

- **`q0`:** before any rules.
- **`q1`:** after the first two rules. It runs their witnesses side by side:
  one edge per signer, and both signatures for rule changes.
- **`q2`:** after `no_shouting`. Every edge here also carries `-shouting`.

For a rule of the form `always(p)`, the per-rule witness has one edge for
each prime implicant of `p`. That is the most permissive model the rule
allows. The synthesizer keeps a candidate only if it passes the same
validation as a hand-written model.

```
     refused    alice says HELLO
               no transition from [q2] matches the commit (nearest edge wants -shouting, got true)
     refused    alice says 1234 (shouting is unknown)
               no transition from [q2] matches the commit (nearest edge wants -shouting, got unknown)
  #9 accepted   alice says bye  (model at [q2])
```

"1234" has no letters, so `shouting` returns **unknown**. Unknown never
satisfies a label, so a commit the model can't judge is refused.

## 8. State, and the log

```
  said so far: [(alice, 3), (bob, 1), (carol, 1)]; chatty(alice) = true
```

[`chatty`](../../AliceBob.lean#L68) reads the accumulated state, not just
the commit: it is true when the author has spoken more than the other party.

```
== The log: a hash chain of signed commits
  #0 f463ff961c6f [alice, bob] ids
  #1 1d965c75dc4e [alice] alice: hello
  #2 df66ee2b7dce [2a776f52…] carol: hey
  #3 62ffc183abcd [alice, bob] rules +rules [either_signs, both_for_rules] +model
  ...
  #9 112e123f2c60 [alice] alice: bye

Rules: [either_signs@#3, both_for_rules@#3, no_shouting@#8]
```

Each commit's id is the hash of its signed payload and its signatures. The
payload names the previous id, so changing any past commit would change
every id after it. Signers appear under their bound names. Carol has no
name, so she appears as a key fingerprint. Each rule is shown with the
commit that anchors it.

## Try changing it

- **Make Alice's model Bob-friendly.** Add Bob's edge to `aliceModel` and
  watch scene 4 change.
- **Break Bob's model.** Delete its `+adds_rule` edge and run again. The
  checker should refuse it before commit `#5`, and say why.
- **Add another rule.** Write your own predicate and rule, then hand it to
  `Synth.forContract`.
