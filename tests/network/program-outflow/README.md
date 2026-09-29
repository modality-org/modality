# Outflow only through a posted program

A treasury posts a payout program and a model whose only `SEND` edge needs
`+emitted_by(/__programs__/payout.wasm, "<sha256>")`. Rules keep it that way:
every `SEND` must be the program's output, and the program never changes.

- The owner's hand-written `SEND` is refused locally and on the sequencer,
  though the owner signs it.
- An `invoke` of the program, whose output is that `SEND`, is sequenced.
- A stranger replays the prefix, re-runs the program, and accepts it.

```bash
./test.sh
```

Unnumbered: not part of the stable numbered suite.
