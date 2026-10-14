# Hybrid devnet1 smoke test

`./test.sh` starts one isolated `run-hybrid` node with a local two-block epoch,
SHA-256 mining, and no bootstrap peers. It waits for mined blocks, the epoch-2
transition, selection from epoch-0 nominations, and the Shoal loop. The test
has bounded waits and runs in Rust CI. It does **not** prove multi-node
propagation or public-testnet finality.

`./01-run-hybrid-node.sh` runs the same short-epoch fixture manually. It creates
`tmp/node1` from scratch; do not point it at an existing node directory.
