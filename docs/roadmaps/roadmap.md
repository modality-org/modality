# Network implementation checklist

This is a checklist of **public implementation and evidence**, not a claim that
mainnet is ready. A checked item means the linked implementation, configuration,
documentation, or test exists in this repository. It does not establish that
the public hosts are healthy today.
For live access, start with [Join the public testnet](../cli/join-testnet.md).

## Join and boot

- [x] The bundled [testnet network config](../../rust/modality-networks/networks/testnet/info.json)
  names three bootstrap peers, a named bootstrap validator set, predicate theory
  `v3`, and development MOD issuance. This is not the mainnet schedule or an
  independent validator set.
- [x] `modal node create --testnet` creates a fresh joiner identity; the
  [join guide](../cli/join-testnet.md) covers miner and hybrid startup and
  second-directory ping.
- [x] Node creation uses `node.modal_passfile`, as documented under
  [Node Commands](../cli/node-commands.md); the numbered
  [ping test](../../tests/network/01-ping-node/test.sh) checks that filename.
- [ ] A fresh external operator has published a dated successful join, ping,
  and mined-block trace using the current release. Configuration and local
  tests alone do not prove public reachability.

## Mining, ordering, and verification

- [x] The Rust workspace contains the
  [miner](../../rust/modality-miner/),
  [node](../../rust/modality-node/),
  [sequencer](../../rust/modality-sequencer/), and
  [validator](../../rust/modality-validator/) crates. `run-sequencer` orders;
  `run-validator` performs the separate prefix-certificate role.
- [x] The [CI workflow](../../.github/workflows/ci.yml) runs workspace
  `cargo check --all`, `cargo check --all --tests --examples`, `cargo test --all`,
  and workspace Clippy. These cover network crates as workspace members. CI also
  builds `modal` and runs the numbered `01-ping-node` and
  `02-run-devnet1` scenarios: the first boots and pings from a second node,
  and the second certifies a local static-sequencer round. The remaining
  numbered scenarios are not yet CI gates.
- [x] A small [hybrid consensus experiment](../../experiments/hybrid-consensus/README.md)
  contains TLA+ and Lean artifacts. It is not a proof of the entire deployed
  protocol.
- [ ] Publish a repeatable multi-node run showing block propagation,
  epoch-based sequencer nomination, finality, and recovery after a peer loss.
- [ ] Demonstrate independent mining operators and non-pool nodes on the
  public network; Foundation bootstrappers alone do not establish this.

## Contracts, observation, and MOD

- [x] The [testnet tutorial](../tutorials/on-the-testnet.md) and
  [join guide](../cli/join-testnet.md) describe posting a contract through a
  sequencer. The [verifier rejection reference](../reference/verifier-rejections.md)
  describes checks that can reject a modeled commit.
- [x] The public [observer/explorer instructions](../cli/join-testnet.md)
  identify the node0 instance and say another observer can serve the same HTTP
  interface. A documented endpoint is not an uptime guarantee.
- [x] The [wallet commands](../cli/wallet-commands.md) document creating a
  wallet, receiving mined MOD, querying balance, and taking testnet faucet MOD.
- [ ] Publish a dated end-to-end trace from an external node: mine or faucet,
  receive and query MOD, post a governed contract, and inspect its history from
  an independently running observer.

## Reproduce and update this checklist

From `rust/`, run the commands in [the CI workflow](../../.github/workflows/ci.yml).
Use [the numbered network scenarios](../../tests/network/) for local boot
and propagation checks. Mark an open item complete only with a linked run
artifact or reproducible automated test; update claims when the network
configuration or release changes.
