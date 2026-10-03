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

## What the tools accept today

- Anywhere you type an ID (`--signer`, `--payer`, `--to`, `--to-contract`,
  `--asset-contract`, `--contract-id`, `--owner`, `--node-id`, a `.id`
  value given to `modal c commit --post` or `modal c set`, a `.id` file),
  any of the forms above is accepted.
- The tools rewrite it to base58 before it is stored or signed. Contract
  state, commit signatures, and network records hold base58 until the
  network switches its stored form to the Modality form; a network that
  switches says so in its release notes.
- One key has one spelling inside a contract. Rules match signers by their
  ID as text, so a network accepts each key in its stored form only, never
  a respelling.
- Hub contracts named `c_…` are not keys and are passed through as given.
- A `/p2p/` part of a multiaddr must still be base58: libp2p's multiaddr
  parser accepts no other form there.
