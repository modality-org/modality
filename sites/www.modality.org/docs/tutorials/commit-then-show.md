---
sidebar_position: 15
title: Commit, Then Show
---

# Commit, Then Show

You can fix a commit's place before you show its body. The hash lane orders
the hash. The body comes later, as a reveal, and it has to be the body that
hashes to that id. A reveal of different bytes is refused.

Do this on the testnet from [On the testnet](on-the-testnet), or any
network whose config has a `hash_lane`. A network without one refuses
`anchor`. The contract in `./on-testnet` is enough. Create it with the
signers who may anchor, if you want other keys kept off the lane:

```bash
modal contract create \
  --dir ./sealed \
  --signer ~/.modality/passfiles/example/alice.mod_passfile
```

Finish the bootstrap the way [On the testnet](on-the-testnet) does: post
Alice, add `always([-signed_by(/parties/alice.id)] false)`, write the
one-signer witness, and commit. Then write the terms locally and commit
them, and do not push the body yet.

```bash
cd sealed
modal commit \
  --path /terms.text \
  --value "10 units, opened later" \
  --sign example/alice \
  -m "Sealed terms"
modal contract anchor --sign example/alice --remote /dns4/node1.testnet.modality.network/tcp/4040/ws/p2p/<node1 peer id>
```

`anchor` signs a hash commitment for each commit not yet pushed, grinds the
hashtax against the node's epoch anchor, and submits it. The record carries
the contract id, the commit hash, the parent hash, and the signature. It
carries no body. Ask the node what it has:

```bash
modal contract anchor --status --commit <commit id> --remote <the same multiaddr>
```

`anchored` means the hash is ordered. The terms are not state yet. When you
are ready to show them:

```bash
modal contract push --reveal --remote <the same multiaddr>
```

The node checks four things, in order. The body hashes to the commit id.
A certified hash commitment names that commit. The commit extends the
contract's head. The contract's model and rules accept it. A body that does
not hash to the anchored id fails the first check and is refused. The hash
stays in the log as an anchor to bytes that were refused. Nothing in the
contract's state changed.

A commit pushed without `--reveal` does not need a hash commitment. Pull
and replay return bodies. A hash with no body is not a commit they can
return, and a repost of a source commit that was only anchored is refused.

## The idea

The hash lane orders an id. The rules run when the body arrives. Sealed
terms are that split: the other party can see that you committed to some
bytes, and they see the bytes when you reveal them. The full account is
[Hash commitments](../concepts/hash-commitments).

Next: [The program and the invariant](constant-product-pool).
