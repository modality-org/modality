# A constant-product pool

Two token contracts mint `tokA` and `tokB` and send them to an LP and a
trader. A pool contract posts the program in
`examples/programs/constant-product-pool`, creates its `lp` shares, and
locks itself with rules: only the program's output moves its assets, the
reserves follow what moved, payouts go to whoever paid in, a swap keeps the
fee-adjusted product, and no commit lowers the product per share. The
walkthrough is `docs/tutorials/constant-product-pool.md`.

On a sequencer that runs predicate theory v2:

- The LP sends 1000 A and 4000 B with memo `{"op":"add"}` and invokes
  `add`; it receives 2000 shares, stating the amount.
- The trader sends 100 A with `{"op":"swap","min_out":300}`. Mallory
  invokes the swap claiming the `SEND` was 1000; the sequencer refuses it,
  naming the amount. Her honest invoke is sequenced, and the trader receives
  362 B.
- A hand-written payout from the pool to Mallory is refused locally and on
  the sequencer.
- The trader sends 100 A with `min_out` 400, which the pool cannot meet.
  Mallory's invoke claiming `min_out` 0 is refused, naming the memo; the
  trader's own invoke returns the 100 A.
- The LP sends 1000 shares back with `{"op":"remove"}` and receives 550 A
  and 1819 B, half of each reserve rounded down. Mallory, working from the
  head before the remove, invokes a refund of the same `SEND`; the sequencer
  refuses a second child of that head.
- A stranger replays the pool and re-runs every invoke.

Every payout is checked by the receiving wallet's `RECV`, which states the
amount; apply refuses it if the pool paid anything else.

```bash
./test.sh
```

Unnumbered: not part of the stable numbered suite.
