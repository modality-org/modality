# Sequenced rule reject

Shows the loop: local commit → node → sequencer log → accept or reject for the
same reasons as local verify → pull back.

A commit that local `modal commit` rejects (unsigned after a signed-by model)
must not be sequenced, even if it is pushed. A signed commit must be sequenced
and come back on `modal contract pull`.

```bash
rebuild   # cargo build --package modal
./test.sh
```
