# Synthesis Review Bundles

`modality model synthesize` is review assistance. The rule remains the
authority, and the model is only a witness that the current synthesis path found.

For parser-backed rules, ask the CLI to verify the witness and write a review
bundle:

```bash
modality model synthesize \
  --rule rules/post-requires-reviewer.modality \
  --verify \
  --review-bundle review/post-requires-reviewer.md \
  -o model/post-requires-reviewer.modality
```

A useful bundle should let a reviewer answer five questions before signing or
committing the result:

- Which rule file was used?
- Which action labels and predicate calls were extracted by the parser?
- Did `--verify` accept the witness model?
- Which assumptions and known gaps are still outside the proof?
- Does the witness model expose only the moves the rule intended?

## Passing Bundle

For a rule such as:

```modality
rule post_requires_reviewer {
  formula {
    always(!+POST | <+signed_by(/users/reviewer.id)> true)
  }
}
```

the bundle should include the rule source, extracted facts such as `+POST` and
`+signed_by(/users/reviewer.id)`, a passed verifier result, and the witness
model that the verifier accepted. Treat that witness as something to inspect,
not as proof that the original human intent was complete.

## No-Witness Bundle

If `--verify` rejects the synthesized candidate, the CLI should say that no satisfying witness was found by the current synthesis heuristics. With `--review-bundle`, it should still write a failed bundle containing:

- The rule file and parser-backed extracted facts.
- The verifier error.
- The candidate witness model that failed verification.
- Known gaps, including the bounded heuristic search path.

This is not a contract approval. It is a review artifact that says the current
tooling did not find a satisfying witness. Revise the rule, supply a witness
model manually, or improve the synthesizer before treating the rule as ready.

## Review Boundary

The current parser-backed path covers rule formulas and generated witness
models. It does not automatically prove that natural-language intent,
real-world evidence, or external predicates are complete. When a bundle mentions
assumptions, keep them visible in the contract review instead of hiding them in
the generated model.
