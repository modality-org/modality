# Hash lane: anchor a commit's hash, reveal its body later

A devnet1 sequencer whose `network.json` turns on the hash lane with a quota
of 2 hash commitments per sequencer block and an 8-bit hashtax floor:

- `modal contract anchor` signs a hash commitment for each unpushed commit,
  grinds its hashtax, and submits it. A certified sequencer block anchors
  the hash. The node holds no body: `modal contract pull` returns nothing.
- `modal contract push --reveal` pushes the bodies. Each is sequenced only if
  it hashes to a certified hash commitment at the same parent.
- A reveal of a commit that was never anchored is refused.
- A reveal whose body was edited after anchoring is refused; the true body
  is then sequenced.
- Five commits anchored at once land at most 2 per block; the rest wait for
  later rounds.
- A REPOST whose source commit was only anchored is refused.
- Alice anchors a contract's genesis with `--signer` herself. A hash
  commitment for it signed by Mallory is refused; the same commit signed by
  Alice is anchored.
- After a restart the node still reports what it anchored and revealed.

```bash
./test.sh
```

Unnumbered: not part of the stable numbered suite.
