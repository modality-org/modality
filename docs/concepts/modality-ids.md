---
sidebar_position: 9
title: Modality IDs
---

# Modality IDs

The format is specified in [RFC-002: RCIDs](../rfcs/RFC-002-RCID.md).

A Modality ID names an ed25519 public key. Contracts, signers, members,
nodes, and validators are all named by one. It is a libp2p peer ID: the
key's bytes behind a fixed header that says "identity multihash of an
ed25519 key".

## One key, several spellings

libp2p writes the same peer ID in more than one way. For one key:

| Form | Text |
|------|------|
| base58 multihash (libp2p's legacy form) | `12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd` |
| CIDv1, base32 (libp2p's newer form) | `bafzaajaiaejcaabcornvmd5g5bolzmcr6smsje7owdd3xk7lojqydmthdt47iqmi` |
| CIDv1, base36 | `k51qzi5uqu5d…` |
| **Modality spelling** | `imqi74tdhtmdyqjol7kx3ddwo7ejsms6rcmzlob5g5dmvnrocbaacjeaiajaazfab` |

The Modality spelling is the lowercase CIDv1 base32 form, mirrored so the
type is at the end of the string. Forward forms open with a fixed type
header (`12D3KooW`, `bafzaajaiaejc`, `k51qzi5uqu5d`). Mirrored, that type
is the tail (`…aiajaazfab`) and the key comes first, so IDs differ from
character one.

**The Modality spelling is the standard Modality ID.** Shorten it from the
front (`imqi74td…`), never by keeping its end: every Modality ID ends in
`…aiajaazfab`.

## Where each form is used

- **`.id` values** in contract state hold the Modality spelling. `modal id
  create` prints it and writes it to `~/.modality/ids/<name>.id`;
  `modal set-named-id`, `modal c set`, `modal c commit --post`, and
  `modal c create --signer` write it. A `.id` value posted earlier in base58
  stays valid.
- **Contract IDs** are the Modality spelling. `modal contract create` prints
  that spelling, and the genesis commit stores it. A contract created
  earlier keeps the spelling its signatures cover. Lookups take either
  spelling of the same key.
- The keys of `head.signatures` and `head.payer` stay base58.
- **A node's status page** shows node and contract IDs, and `.id` values,
  in the Modality spelling, shortened from the front. Node links, anchors
  and `status.json` keep base58, so nodes of different versions still match
  each other; `/contracts/<id>` takes either form.
- **A `/p2p/` part of a multiaddr** must be base58: libp2p's multiaddr
  parser accepts no other form there.
- Anywhere you type an ID (`--signer`, `--payer`, `--to`, `--to-contract`,
  `--asset-contract`, `--contract-id`, `--owner`, `--node-id`, a `.id`
  value, a `.id` file), any form above is accepted and rewritten to the one
  that place uses. Hub contracts named `c_…` are not keys and pass through
  as given.

## Rules compare keys, not spellings

`signed_by`, `any_signed`, `all_signed`, `threshold`, `posts_own_key`, and
`sent_to` read an ID from state and compare it with a commit's signers or
sends. They compare the key each one names, so a signature keyed in base58
matches a `.id` value in the Modality spelling, and a 32-byte ed25519 key in
hex matches both. One key is one signer: signing twice under two spellings
counts once toward a threshold, and respelling a key does not get it past
`-signed_by`.
