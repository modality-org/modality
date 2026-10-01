---
sidebar_position: 8
title: Wallet Commands
---

# Wallet Commands (`modal wallet`)

A wallet is a contract whose id is your key's id and which only that key can
extend. Its genesis names the key as the one signer (`/signers/1.id`) with
the rule `always([-signed_by(/signers/1.id)] false)`, so the network refuses
any commit your key did not sign. The signer is fixed when the wallet is
made: keep the key, or the wallet is lost with it.

Your address is your key's id. Anyone can send to it before your wallet is
on the network. Receiving is a commit: one `RECV` per `SEND`, stating what
it takes, signed by your key. `modal wallet recv` writes them for you.

Full wrapper only (`cargo build -p modal`).

## Create

```bash
modal wallet create [--key <PASSFILE>] [--dir <DIR>] [--remote <MULTIADDR>]
```

| Option | Description |
|--------|-------------|
| `--key <PASSFILE>` | Your key: a passfile path or identity name. Default: a new key written to `<dir>/owner.mod_passfile` |
| `--dir <DIR>` | Wallet directory. Default: `~/.modality/wallet` (`$MODALITY_HOME/.modality/wallet` when set) |
| `--remote <MULTIADDR>` | The node the wallet reads from and pushes to. Default: a testnet bootstrapper |
| `--output <FORMAT>` | `text` or `json` |

If the network already holds a contract at your key's id, `create` copies it
instead of making a second genesis, which the network would refuse. Run it
with the same key on a new machine to get your wallet back.

A miner's blocks nominate its node's peer id, so the node's key makes the
wallet that holds its MOD:

```bash
modal wallet create --key ./my-node/node.modal_passfile --dir ./my-node-wallet
```

## Address, balance, incoming

```bash
modal wallet address
modal wallet balance
modal wallet incoming
```

`balance` shows what the wallet holds, in whole units when the asset gives
`decimals`, and how many sends wait to be received. `incoming` lists each
waiting `SEND`: amount, asset, sender, and the `SEND` commit and index. Both
read the remote node's `/contract/account`, so a send counts once it is
sequenced there.

## Receive

```bash
modal wallet recv
```

Makes one `RECV` for each waiting `SEND`, signed by your key, then pushes
every commit the remote has not taken (the genesis included, the first
time). A send this copy already receives is skipped, so running `recv` again
before the network sequences the first batch does not receive twice.

## Send

```bash
modal wallet send --to <CONTRACT_ID> --amount <AMOUNT> [--asset MOD]
modal wallet send --to <CONTRACT_ID> --amount 2.5 --asset tok --asset-contract <CREATOR_ID>
```

| Option | Description |
|--------|-------------|
| `--to <CONTRACT_ID>` | Who to pay |
| `--amount <AMOUNT>` | In whole units, to the asset's `decimals` (e.g. `1.5`) |
| `--asset <ASSET>` | `MOD` (the default), or an asset id with `--asset-contract` |
| `--asset-contract <ID>` | The contract that created the asset; not needed for MOD |
| `--memo <MEMO>` | A note for the receiver |
| `--tip <UNITS>` | A tip per gas, to be ordered sooner when rounds are full |

`send` refuses an amount the wallet does not hold, one finer than the
asset's decimals, and one that is not a multiple of its divisibility. The
receiver takes the payment with a `RECV` (`modal wallet recv`).

## Common options

`address`, `balance`, `incoming`, `recv` and `send` take `--dir`,
`--remote` (use another node this once) and `--output json`.

See [The MOD Contract](../concepts/mod-contract.md) for how MOD reaches a
miner's address.
