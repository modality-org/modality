# Observer catch-up

An observer that joins after contracts are sequenced pulls their history
from the sequencer, then follows new commits.

- A source contract mints, sends 10 drops to a wallet, and the wallet
  receives them, all before the observer exists.
- The observer starts, pulls the sequenced history, and its explorer API
  (`/api/contracts/<id>`) serves the same balances: 90 at the source,
  10 in the wallet. `/api/contracts/<id>/commits` lists the `RECV` as
  sequenced.
- The source sends 5 more; the observer applies it from certified gossip
  and serves 85.

The devnet does not require a validator QC for dest `RECV`, so this does
not cover an observer applying a `RECV` on a network that does.

```bash
./test.sh
```

Unnumbered: not part of the stable numbered suite.
