# Testnet faucet

A contract that drips 10 testnet MOD, once, to each key that registers with
it. Nobody owns it: its first commit is signed by no one, and its rules say
everything it will ever accept.

- A key is registered at `/claimants/<label>.id` only in a commit that key
  signs.
- A registered key sends exactly `/config/drip.num` of `/config/asset.text`
  once, setting `/claimants/<label>/claimed.bool` in the same commit.
- It mints nothing. Its config, model and rules never change.
- Anyone funds it with a commit that only receives.

Claim with `modal wallet faucet` (docs/cli/wallet-commands.md).

| File | What |
|------|------|
| `create.sh` | Makes a faucet for any held asset; used by `tests/network/faucet` |
| `testnet.json` | The testnet faucet's genesis and first commit, as `create.sh` made them; its id is `faucet_contract` in the testnet's `info.json` |
| `push.sh` | Pushes `testnet.json` to a node, after each new testnet genesis |

## Fund it

From a wallet that holds MOD (for example a Foundation node's, made with its
node key):

```bash
modal wallet send --dir <wallet> --to <faucet id> --amount 1000
# once the SEND is sequenced, receive it into the faucet:
modal pull --contract-id <faucet id> --remote <node> --dir ./faucet
modal commit --all --dir ./faucet --method recv --send-commit-id <SEND commit> \
  --asset-contract <MOD contract id> --asset-id MOD --amount 100000000000
modal contract push --dir ./faucet --remote <node> --remote-name origin
```

`modal wallet faucet` says when the faucet holds less than a drip.

## Limits

- Every new key gets a drip, so the faucet is only as deep as its funding. Fine
  for a testnet. It is not a way to share out anything of value.
- A claim is two commits paid for by the claimant's wallet. While gas is
  unpriced that costs nothing. Once gas is priced, a new wallet with no MOD
  cannot pay to claim; the faucet will then need a sponsor that pays for
  claims.
