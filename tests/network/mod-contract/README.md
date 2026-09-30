# The MOD contract

A network whose MOD is the asset of one genesis contract. The test builds
the genesis with `scripts/mod-genesis/build.sh`: 21,000,000 MOD at
divisibility `10^8`, 50 MOD a block halving every 4 blocks, and two
allocations. The foundation's key signs the bootstrap, the `CREATE`, the
allocations and the rules; after that only the emission program's output
is accepted.

- A node refuses a genesis commit edited after it was named, a MOD contract
  on a network below predicate theory `v2`, and a network that also names
  `emission`.
- A sequencer applies the 6 genesis commits and takes its emission from the
  contract's posts.
- Alice's wallet receives her allocation, stating the amount; a `RECV` that
  overstates Bob's is refused.
- Locally, the program's mint of block 1 passes the rules, while a
  hand-written `SEND` by the foundation's key, and paying block 1 twice,
  are refused.
- The sequencer refuses the mint when a client pushes it: only the network
  writes the MOD contract.
- A stranger replays the MOD contract from the sequencer.
- After a restart, no genesis commit is applied again.

```bash
./test.sh
```

Unnumbered: not part of the stable numbered suite.
