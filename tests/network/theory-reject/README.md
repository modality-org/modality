# Theory reject

A network can enforce predicate theory V2 by setting
`"predicate_theory_version": "v2"` in its `info.json`. This test runs a
devnet1 sequencer with that one field changed.

Alice sells Bob a laptop for 100 through an escrow model. The refund edge was
copied from the release edge and still says `num_gte(/escrow/paid.num,"100")`
next to `num_lt(/escrow/paid.num,"100")`, so no commit can ever take it.

1. `modal commit` accepts the model (local verify uses V0) and prints a V2
   preview; `modal contract theory` names the dead edge.
2. The sequencer refuses the pushed commit with the same explanation, and
   `modal contract pull` does not return it.
3. With the stray `num_gte` deleted, the model is sequenced and pulled.

```bash
rebuild   # cargo build --package modal
./test.sh
```
