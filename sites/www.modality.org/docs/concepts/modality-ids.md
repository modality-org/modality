---
sidebar_position: 9
title: Modality IDs
---

# Modality IDs

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
| **Modality form**: the base32 CID, backwards | `imqi74tdhtmdyqjol7kx3ddwo7ejsms6rcmzlob5g5dmvnrocbaacjeaiajaazfab` |

The fixed header makes every forward form begin with the same characters
for every key: `12D3KooW`, `bafzaajaiaejc`, `k51qzi5uqu5d`. Written
backwards, the base32 CID begins with the key and ends with the header, so
its first characters differ from ID to ID and it is all lowercase.

**The Modality form is the standard Modality ID.** Shorten it from the
front (`imqi74td…`), never by keeping its end: every Modality ID ends in
`…aiajaazfab`.

## Where each form is used

- **`.id` values** in contract state hold the Modality form. `modal id
  create` prints it and writes it to `~/.modality/ids/<name>.id`;
  `modal set-named-id`, `modal c set`, `modal c commit --post`, and
  `modal c create --signer` write it. A `.id` value posted earlier in base58
  stays valid.
- **Contract IDs** and the keys of `head.signatures` are base58, and so is
  `head.payer`.
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
matches a `.id` value in the Modality form, and a 32-byte ed25519 key in
hex matches both. One key is one signer: signing twice under two spellings
counts once toward a threshold, and respelling a key does not get it past
`-signed_by`.
