---
sidebar_position: 11
title: On the Testnet
---

# On the Testnet

A commit on your disk is yours. A sequenced commit is one a stranger can
open. You push the log to the public testnet, watch it sequence, and read
the contract id in the Modality spelling.

The testnet is the network in [Join the public testnet](../cli/join-testnet).
It runs predicate theory v3. You need a `modal` that includes node
commands. Status and the explorer:

- `https://testnet.modality.network`
- `https://node0.testnet.modality.network`

Use a contract you already trust locally, or this small one. The rule is
the first-contract rule, so an unsigned commit is refused before a push
can send it.

```bash
modal contract create --dir ./on-testnet
cd on-testnet
modal checkout
modal set-named-id /parties/alice.id example/alice
modal add-rule --name authorized \
  'always([-signed_by(/parties/alice.id)] false)'
```

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/parties/alice.id)
  }
}
```

```bash
modal commit --all -m "Bootstrap"
modal commit --path /data/message.text --value hello -m "Unsigned"
```

```output
No valid transition for local commit from current states {"q1"}
Closest candidate transition: q1 -> q1 [+signed_by(/parties/alice.id)]; failed predicates: missing +signed_by(/parties/alice.id)
```

That commit is not in the log, so there is nothing bad to push. Sign the
message, then push to a bootstrapper. The peer id in the multiaddr is the
one `modal net info testnet` prints for node1.

```bash
modal commit --path /data/message.text --value hello --sign example/alice -m "Hello"
modal contract push --remote /dns4/node1.testnet.modality.network/tcp/4040/ws/p2p/<node1 peer id>
```

`modal status` then shows a remote head once the push is accepted. The
contract id on that page is the Modality spelling: lowercase, mirrored, and
it ends in `aiajaazfab`. The explorer takes that spelling or the base58
spelling of the same key:

`https://node0.testnet.modality.network/contracts/<contract id>`

Open it. The hello commit is in the history a stranger can read. Signature
keys inside the commit stay base58. The contract id does not.

## The idea

Local acceptance is your checker. Sequencing is the network ordering the
commit so another machine can pull it and replay it. A push of a commit the
witness refuses never starts, because `commit` already left it out of the
log. The id you paste into the explorer is the
[Modality spelling](../concepts/modality-ids).

Next: [Get paid](get-paid).
