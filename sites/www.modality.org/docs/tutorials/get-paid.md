---
sidebar_position: 12
title: Get Paid
---

# Get Paid

A wallet is a contract whose id is your key, and only that key can extend
it. Your address is that id, in the Modality spelling. Someone can send to
it before the wallet is on the network. Receiving is a commit you sign.

This continues [On the testnet](on-the-testnet). MOD on that network is the
genesis contract's asset, paid to a miner. The commands are
[wallet commands](../cli/wallet-commands). Full `modal` only.

Make the wallet from a key you control. A miner's MOD arrives at the node's
key, so a mining node uses its `node.modal_passfile`. Anyone else uses a
passfile of their own.

```bash
modal wallet create --key example/alice --dir ./alice-wallet
modal wallet address --dir ./alice-wallet
```

The address is Alice's key. Ask the other party, or your own second wallet,
to send a testnet amount of MOD to it. From a wallet that already holds MOD:

```bash
modal wallet send \
  --dir ./bob-wallet \
  --to <alice's address> \
  --amount 1 \
  --asset MOD
```

`send` refuses an amount the wallet does not hold, an amount finer than the
asset's decimals, and an amount that is not a multiple of its divisibility.
That refusal happens before a commit is pushed.

Alice lists what is waiting, then receives it. `recv` writes one `RECV` per
waiting send, signed by her key, and pushes.

```bash
modal wallet incoming --dir ./alice-wallet
modal wallet recv --dir ./alice-wallet
modal wallet balance --dir ./alice-wallet
```

A second `recv` before the first batch has sequenced does not receive the
same send twice. A receive that names a send which was not to this wallet,
or states a different amount, is refused when the network applies it. The
waiting list is the set of sends that apply will accept.

## The idea

The wallet's genesis posts its key at `/signers/1.id` and the rule
`always([-signed_by(/signers/1.id)] false)`. Sending and receiving MOD are
commits on that contract, checked like any other commit. The address you
pay is the contract id. Decimals are display: an `--amount` of `1` is one
whole MOD, which the chain stores in the asset's smallest unit.

Next: [Someone else's word](oracle-escrow).
