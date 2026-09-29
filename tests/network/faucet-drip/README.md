# Faucet: a fixed drip, once, to registered holders

A faucet contract on a sequencer that enforces predicate theory `v2`. The
founder mints a test asset once. After that the model and rules decide
every move. Every faucet commit uses `modal commit --theory v2`, so local
verify matches the network; two of the rules are ones `v0` refuses.

- A stranger registers a fresh slot with a key she holds
  (`+posts_own_key(/claimants/$k.id)`); Mallory cannot register Dave's key.
- A claimant drips once, alone, exactly `/config/drip.num`
  (`+sent_eq("drops", /config/drip.num)`), setting her own flag true in the
  same commit (`+post_to(/claimants/$k/claimed.bool, "true")`, with
  `modal commit --all --method send ...`). A drip that writes the flag
  `false` would leave the next drip open, so it is refused.
- An oversized drip is refused locally, and on the sequencer when a client
  skips local verify. A second drip is refused.
- Carol's wallet `RECV`s the sequenced drip; a second `RECV` of the same
  drip is refused.
- A stranger replays the log and accepts the drip.

```bash
./test.sh
```

Unnumbered: not part of the stable numbered suite.
