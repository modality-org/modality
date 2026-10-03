# RFC-002: RCIDs — Reversed CIDs as the Modality Spelling of an ID

**Status:** Accepted (implemented in `modal`)  
**Author:** Foy Savas  
**Created:** 2026-10-03  

## Summary

A Modality ID names an ed25519 key and is a libp2p peer ID. This RFC fixes
how Modality writes one: as an **RCID**, the peer ID's lowercase CIDv1
base32 form, reversed.

```
base58 multihash:  12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd
CIDv1, base32:     bafzaajaiaejcaabcornvmd5g5bolzmcr6smsje7owdd3xk7lojqydmthdt47iqmi
RCID:              imqi74tdhtmdyqjol7kx3ddwo7ejsms6rcmzlob5g5dmvnrocbaacjeaiajaazfab
```

All three name the same key. An RCID starts with the key and ends with
the type, so two IDs differ from their first character, and a short prefix
tells them apart.

## Motivation

Every text form libp2p defines starts with a fixed header. For an ed25519
key, the bytes are always `00 24 08 01 12 20` (identity multihash,
protobuf key type) followed by the 32-byte key. So:

| Form | Fixed prefix, every key |
|------|-------------------------|
| base58 multihash | `12D3KooW` (8 characters) |
| CIDv1, base36 | `k51qzi5uqu5d` (12) |
| CIDv1, base32 | `bafzaajaiaejc` (13) |

People shorten IDs to their first characters: in logs, tables, status
pages, chat. With these forms, every shortened ID reads `12D3KooW…`. Keeping
start and end (`12D3KooW…wGxxHd`) spends half the space on nothing.

libp2p will not change the bytes. They are frozen across implementations
and deployments: the move to CIDv1 text output in rust-libp2p
([#2322](https://github.com/libp2p/rust-libp2p/pull/2322)) was closed over
compatibility, and libp2p implementations still print base58. Nor can a
key be ground to avoid the prefix: the prefix is not part of the key.

Reversing the base32 CID moves the fixed header to the end. Nothing is
added or removed, so an RCID is still a libp2p peer ID, one string
reversal away from the form the libp2p spec defines
([RFC 0001](https://github.com/libp2p/specs/blob/master/RFC/0001-text-peerid-cid.md)).

## Specification

### Encoding

For a peer ID with bytes `P` (its multihash):

```
RCID(P) = reverse( "b" || base32lower_nopad( 0x01 || 0x72 || P ) )
```

- `0x01` is CID version 1; `0x72` is the `libp2p-key` multicodec.
- `base32lower_nopad` is RFC 4648 base32, lowercase, no padding.
- `"b"` is the multibase prefix for that encoding.
- `reverse` reverses the characters.

An ed25519 RCID is always 65 characters from `a–z` and `2–7`. Its first
character takes all 32 values. Its last 13 characters are always
`cjeaiajaazfab`, and the one before them is `a` or `b`.

Only the base32 CID is reversed. A reversed base36 or base58 CID is not an
RCID.

### Test vector

| | |
|---|---|
| ed25519 public key (hex) | `0022745b560fa6e85cbcb051f4992493eeb0c7bbabeb726181b2671cf9f44188` |
| base58 multihash | `12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd` |
| CIDv1, base32 | `bafzaajaiaejcaabcornvmd5g5bolzmcr6smsje7owdd3xk7lojqydmthdt47iqmi` |
| RCID | `imqi74tdhtmdyqjol7kx3ddwo7ejsms6rcmzlob5g5dmvnrocbaacjeaiajaazfab` |

### Parsing

A Modality tool that reads an ID accepts every form, in this order:

1. Text starting with `1` or `Qm` is a base58 multihash (libp2p spec).
2. Otherwise, a multibase-encoded CIDv1 whose codec is `libp2p-key`.
3. Otherwise, if the text ends in `b`, the same read reversed: an RCID.

Anything else, and any CID with another codec, is not a peer ID. Text
that is not a peer ID passes through unchanged where a place takes other
names too (hub contracts named `c_…`).

### Display

Shorten an RCID from the front: `imqi74td…`. Never keep its end; every
RCID ends the same way. Eight characters carry 40 bits, enough to tell
apart the IDs on any page.

### Equality

One key has several spellings: the forms above, and, where Modality
accepts a raw key, 64 hex characters. Modality never compares IDs as text.
Both sides are reduced to the RCID of the key they name, then compared.

This matters for rules. `signed_by`, `any_signed`, `all_signed`,
`threshold`, `posts_own_key`, and `sent_to` read an ID from contract state
and compare it with a commit's signers or sends. Compared as text, a key
signed under a second spelling would be a second signer: it could count
twice toward a threshold, or get past `-signed_by`. Compared as keys, it
is one signer.

### Where each form is used

| Place | Form |
|-------|------|
| `.id` values in contract state | RCID. Validators also accept base58, as posted before this RFC. |
| Contract IDs | RCID for contracts created from this RFC on. A contract keeps the spelling it was created under: its genesis stores it and its signatures cover it. Lookups take either spelling of the same key. |
| `modal id create` output, `~/.modality/ids/*.id` | RCID |
| A node's status page | RCID, shortened from the front |
| Keys of `head.signatures`, `head.payer` | base58 |
| A node's `status.json`, page links and anchors | base58, so nodes of different versions still match each other |
| `/p2p/` in a multiaddr | base58: libp2p's multiaddr parser takes no other form |
| Anything a person types (`--signer`, `--to`, `--contract-id`, …) | any form, rewritten to the one that place uses |

## Compatibility

- **libp2p.** An RCID reversed is a valid libp2p CIDv1 peer ID. Modality
  hands libp2p base58 or the forward CID.
- **Existing contracts.** Their base58 `.id` values and contract IDs stay
  valid. Because rules compare keys, base58 and RCID values of the same key
  match each other.
- **Networks.** A node that predates this RFC refuses a `.id` value holding
  an RCID. Nodes on one network upgrade together. Replaying existing
  history gives the same verdicts, with one exception: a commit that got
  past `-signed_by` by signing under a hex spelling of the same key is now
  refused.

## Security considerations

- **Respelling.** Before this change, signatures could be keyed in hex
  while rules compared IDs as text, so a hex-keyed signature could get past
  `-signed_by` on a base58 `.id` value. Comparing keys closes that.
- **Duplicate signatures.** Two entries of `head.signatures` under two
  spellings of one key are one signer.
- **Signed text.** A contract ID is part of every signing payload as text.
  Respelling an existing contract's ID would change what its signatures
  cover, so contracts keep their spelling.

## Alternatives considered

- **Show base58 without its prefix** (`9pte76rp…`). It works for display
  only. A person copying the shortened ID gets something no tool reads.
- **base58 of the raw 32-byte key** (Solana style, 43–44 characters). Not
  a libp2p peer ID, so every conversion needs a rule outside libp2p.
- **Forward CIDv1.** A longer fixed prefix than base58 (12–13 characters).
- **Reversed base36 CID** (62 characters). It fits in one DNS label (63
  characters); a base32 RCID (65) does not. Base32 was chosen because it
  is libp2p's default CID text form and is bit-aligned.
- **A different key type.** secp256k1 IDs have their own fixed prefix
  (`16Uiu2HAm…`). Hashed key types (`Qm…`) lose the inline key that lets an
  ID verify a signature.

## Open questions

- Whether `head.signatures` keys and `head.payer` move to RCIDs.
- Whether a reversed base36 CID (62 characters) is needed where an ID
  must fit one DNS label.
- Server-side inputs (hub REST and RPC, node requests) still take contract
  IDs as given.
- The `oracle_attests` predicate compares an attestation's hex key with a
  `.id` value as text, so it cannot match; it needs the same key
  comparison.

## Reference implementation

- `rust/modality-common/src/peer_id.rs`: parsing every form, the RCID
  (`modality_peer_id`, `id_value`), and `key_form` for comparing keys.
- Rule checks: `model_governance.rs`, `theory_state.rs`,
  `contract_store/one_step_rule.rs`, `hash_commitment.rs`.
- Status page: `rust/modality-node/src/templates/` (`shown_id`, and
  `modalityId` in the page script).
- User guide: [Modality IDs](../concepts/modality-ids.md).
