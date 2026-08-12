# Synthesis Review Bundles

`modality model synthesize` is review assistance. The rule remains the
authority, and the model is only a witness that the current synthesis path found.

For parser-backed rules, ask the CLI to verify the witness and write a review
bundle:

```bash
modality model synthesize \
  --rule rules/post-requires-reviewer.modality \
  --source-file source/post-requires-reviewer.txt \
  --verify \
  --review-bundle review/post-requires-reviewer.md \
  -o model/post-requires-reviewer.modality
```

A useful bundle should let a reviewer answer five questions before signing or
committing the result:

- Which rule file was used?
- Which reviewer-authored source clause, prompt, or protocol text was preserved?
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

When the rule came from reviewer-authored text, pass that text with
`--source-file` or `--source-text` instead of relying on the rule file alone.
Structured lines such as `F1: Every accepted post move must have reviewer
signature evidence attached.` should appear in the Source Clause Trace section
next to the extracted formula. This trace is preserved for review; it is not
natural-language extraction.

## No-Witness Bundle

If `--verify` rejects the synthesized candidate, the CLI should say that no satisfying witness was found by the current synthesis heuristics. With `--review-bundle`, it should still write a failed bundle containing:

- The rule file and parser-backed extracted facts.
- The verifier error.
- The candidate witness model that failed verification.
- Known gaps, including the bounded heuristic search path.

This is not a contract approval. It is a review artifact that says the current
tooling did not find a satisfying witness. Revise the rule, supply a witness
model manually, or improve the synthesizer before treating the rule as ready.

Review failed bundles with the same care as passing bundles. The failure is
useful only when it preserves enough evidence to diagnose the gap:

- Confirm the rule source is the text the reviewer intended to check.
- Compare extracted facts with the formula and look for missing labels or
  predicates.
- Read the verifier error before changing the rule; it may point at an
  unsupported synthesis pattern instead of an impossible contract.
- Inspect the candidate witness model to see which move the heuristic tried.
- Keep the known gaps attached to the review record when the next revision is
  proposed.

For an intentionally impossible rule such as:

```modality
rule impossible_contract {
  formula {
    false
  }
}
```

a failed bundle should preserve the rule source, state that `--verify` failed,
include the rejected candidate witness model, and name the bounded heuristic
search path as a known gap. That is a useful negative result: it tells reviewers
the tool found no current witness instead of quietly presenting a model as if it
proved the rule.

## Review Boundary

The current parser-backed path covers rule formulas and generated witness
models. It does not automatically prove that natural-language intent,
real-world evidence, or external predicates are complete. When a bundle mentions
assumptions, keep them visible in the contract review instead of hiding them in
the generated model.
