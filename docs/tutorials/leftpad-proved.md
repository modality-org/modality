---
sidebar_position: 20
title: Leftpad, Proved
---

# Leftpad, Proved

The model-checking tutorial ended with leftpad passing a search and an
invariant that held on every state in your domain. That is evidence,
not proof. Here you will prove it for **every** input: read the
obligations, fix an invariant from a counterexample, show the loop
ends, read a proof step by step, and check it a second time in Lean.

You need `modal` and the files from
[Model checking with Modality](/docs/tutorials/model-checking):
`leftpad.modality`, `leftpad.rules.modality`,
`leftpad.invariants.modality` and `leftpad.test.toml`. `domains.toml`
only comes back in section 7. You need Lean only for section 8.
The same files, as they stand at the end of this page, are in
`rust/modality-lang/tests/fixtures/leftpad/`.

These commands are on `main` and not yet in a release binary. Until
they are, build `modal`
[from source](/docs/getting-started/installation#install-from-source).
The test `rust/modality/tests/walkthrough/proving.walk` runs every
`modal` command on this page and checks that it prints what the page
shows.

## 1. From a search to a proof

Here are the model and the invariant as the last tutorial left them,
in full so the line numbers below match. `leftpad.modality`:

```modality
model leftpad {
  initial q0

  // post the inputs: any fill, width and string
  q0 -> q1 [-modifies(/out)]
  // start: result := string, added := 0, pad := max(width - len(string), 0)
  q1 -> q2 [+num_eq(len(/in/fill.text), 1)
            +sets_from(/out/result.text, /in/string.text) +sets(/out/added.natural, 0)
            +sets(/out/pad.natural, max(minus(/in/width.integer, len(/in/string.text)), 0))
            -modifies(/in) -modifies(/out/done.bool)]
  // step: while added < pad, put fill in front of result
  q2 -> q2 [+num_lt(/out/added.natural, /out/pad.natural)
            +sets(/out/result.text, concat(/in/fill.text, /out/result.text))
            +sets(/out/added.natural, plus(/out/added.natural, 1))
            -modifies(/in) -modifies(/out/pad.natural) -modifies(/out/done.bool)]
  // finish
  q2 -> q3 [+num_gte(/out/added.natural, /out/pad.natural) +sets(/out/done.bool, true)
            -modifies(/in) -modifies(/out/result.text)]
}
```

`leftpad.invariants.modality`:

```modality
invariant padded_so_far {
  on q2 -> *
  formula pad_is_set {
    always([-num_eq(/out/pad.natural, max(minus(/in/width.integer, len(/in/string.text)), 0))] false)
  }
  formula length_so_far {
    always([-num_eq(len(/out/result.text), plus(len(/in/string.text), /out/added.natural))] false)
  }
  formula one_char_fill {
    always([-num_eq(len(/in/fill.text), 1)] false)
  }
  formula ends_with_string {
    always([-text_ends_with(/out/result.text, value(/in/string.text))] false)
  }
  formula padded_with_fill {
    always([-text_prefix_repeats(/out/result.text, /out/added.natural, /in/fill.text)] false)
  }
  formula added_le_pad {
    always([-num_lte(/out/added.natural, /out/pad.natural)] false)
  }
}
```

Prove it. There is no domain file: the prover does not try inputs, it
reasons about all of them at once.

```
$ modal prove leftpad.modality leftpad.rules.modality leftpad.invariants.modality
leftpad.modality · leftpad.rules.modality (3 formulas) · leftpad.invariants.modality (6 formulas)
semantics s1 · prover 0.1 · state: any
✓ init      q1 -> q2   meets invariant padded_so_far
✓ type      q1 -> q2   max(minus(width, len(string)), 0) fits /out/pad.natural
✓ type      q2 -> q2   plus(added, 1) fits /out/added.natural
✓ consec    q2 -> q2   keeps invariant padded_so_far
✓ rule      leftpad_spec.length    at q2 -> q3; no other edge can set /out/done.bool
✓ rule      leftpad_spec.padding   at q2 -> q3; no other edge can set /out/done.bool
✓ rule      leftpad_spec.suffix    at q2 -> q3; no other edge can set /out/done.bool
? term      q2 -> q2   unknown: no decreases measure, so the loop may never end
✓ progress  q2         the guards of q2 -> q2 and q2 -> q3 cover every state

8 of 9 proven, 1 unknown                                     exit 2
  = help: add `decreases q2 -> q2 by <term>` to the invariant file
not verified: 1 obligation is not proven
```

The symbols mean what they meant for `modal check`, with a stronger
meaning for ✓:

- **✓ proven.** True for every input and every state the invariant
  allows. The prover has a certificate for it, and checks the
  certificate again on its own before it prints ✓.
- **✗ refuted.** False, and here is a state that shows it. The state
  comes from running the model, so it is never a quirk of the prover.
- **? unknown.** The prover could not decide. It always says why.

Output calls a path by its last name (`width` for `/in/width.integer`)
when that name is unique. `semantics s1` is the meaning of every term
and predicate, the same one `modal run` uses. The last line is the
trust line: `modal prove` prints "verified" only when every obligation
is proven.

## 2. What the prover has to show

Each line above is an **obligation**: one claim, about one edge or one
node, that the proof needs. `--list` lists them:

```
$ modal prove leftpad.modality leftpad.rules.modality leftpad.invariants.modality --list
leftpad.modality · leftpad.rules.modality (3 formulas) · leftpad.invariants.modality (6 formulas)
semantics s1 · prover 0.1 · state: any
  init      q1 -> q2
  type      q1 -> q2
  type      q2 -> q2
  consec    q2 -> q2
  rule      leftpad_spec.length
  rule      leftpad_spec.padding
  rule      leftpad_spec.suffix
  term      q2 -> q2
  progress  q2
```

Three of them are the three questions `--inductive` asked in the last
tutorial, now asked of every state instead of the states in your
domain:

| Obligation | It asks | Last tutorial |
|---|---|---|
| `init` | does the move into `q2` from outside the loop leave the invariant true? | "holds at every q2 state a run reaches" |
| `consec` | does a step from any state the invariant allows leave it true? | "every step keeps it" |
| `rule` | does every edge that can set `done` meet the spec, given the invariant where the move starts? | "strong enough" |
| `type` | does every computed write fit its path's type? | Bug 2 |
| `term` | does the loop end? | — |
| `progress` | can a run at `q2` always take one of its edges? | — |

`init` and `consec` are the same claim about two kinds of edge. Both
`q1 -> q2` and `q2 -> q2` are moves into `q2`, so both must leave
`padded_so_far` true. On `q1 -> q2` the prover assumes nothing about
where the move started. On `q2 -> q2` it may assume the invariant held
before the step. Every edge into `q2` gets an obligation, so no run
reaches `q2` without one; add an edge into `q2` later and it gets its
own.

A `rule` obligation covers **every** edge, not just the finish. Three
edges cannot set `/out/done.bool` at all, because each one promises to
leave it alone (`-modifies(/out)` or `-modifies(/out/done.bool)`). That
is the "no other edge" part. It is why Bug 1 in the last tutorial was a
real bug: an edge that does not promise to leave a path alone may write
anything there.

The invariant is not one of the things being proved about leftpad. The
rules are. The invariant is how the prover gets there, and it never
leaves your machine.

`--only consec` shows just one kind.

## 3. Read a refuted step

The invariant has a formula that looks redundant: `one_char_fill`. The
start edge already refuses a longer `fill`, and no move ever writes
`/in`. Delete it and prove again:

```
$ modal prove leftpad.modality leftpad.rules.modality leftpad.invariants.modality
leftpad.modality · leftpad.rules.modality (3 formulas) · leftpad.invariants.modality (5 formulas)
semantics s1 · prover 0.1 · state: any
✓ init      q1 -> q2   meets invariant padded_so_far
✓ type      q1 -> q2   max(minus(width, len(string)), 0) fits /out/pad.natural
✓ type      q2 -> q2   plus(added, 1) fits /out/added.natural
✗ consec    q2 -> q2   does not keep invariant padded_so_far       refuted
    before: fill = "", string = "", width = 1, result = "", added = 0, pad = 1
            padded_so_far holds; the guard added < pad holds
    after:  added = 1, which breaks 2 formulas:
      padded_so_far.length_so_far      len(result) = 0, but plus(len(string), added) = 1
       --> leftpad.invariants.modality:6:3
      padded_so_far.padded_with_fill   "" has fewer than 1 character
       --> leftpad.invariants.modality:12:3
    = help: q1 -> q2 checks len(fill) = 1, but padded_so_far does not say it
    = replay: modal run leftpad.modality --replay .modal/cex/1.toml
✓ rule      leftpad_spec.length    at q2 -> q3; no other edge can set /out/done.bool
✓ rule      leftpad_spec.padding   at q2 -> q3; no other edge can set /out/done.bool
✓ rule      leftpad_spec.suffix    at q2 -> q3; no other edge can set /out/done.bool
? term      q2 -> q2   unknown: no decreases measure, so the loop may never end
✓ progress  q2         the guards of q2 -> q2 and q2 -> q3 cover every state

7 of 9 proven, 1 refuted, 1 unknown                          exit 1
  = help: add `decreases q2 -> q2 by <term>` to the invariant file
not verified: 2 obligations are not proven
```

No run reaches that `before` state: the start edge would have refused
`fill = ""`. But when the prover checks a step, it does not know how
the run got there. **All it knows at `q2` is the invariant.** Whatever
the start edge checked is forgotten unless the invariant repeats it.
So `one_char_fill` was not redundant. It is how a fact from the start
edge gets to every step.

`--replay` takes the move from that exact state. Put `one_char_fill`
back.

## 4. The frames are part of the proof

The step edge promises `-modifies(/in)`. Drop that promise from line 15
of `leftpad.modality` and prove again:

```
$ modal prove leftpad.modality leftpad.rules.modality leftpad.invariants.modality
leftpad.modality · leftpad.rules.modality (3 formulas) · leftpad.invariants.modality (6 formulas)
semantics s1 · prover 0.1 · state: any
✓ init      q1 -> q2   meets invariant padded_so_far
✓ type      q1 -> q2   max(minus(width, len(string)), 0) fits /out/pad.natural
✓ type      q2 -> q2   plus(added, 1) fits /out/added.natural
✗ consec    q2 -> q2   does not keep invariant padded_so_far       refuted
    before: fill = "a", string = "", width = 1, result = "", added = 0, pad = 1
            padded_so_far holds; the guard added < pad holds
    after:  string = "a", added = 1, result = "a", which breaks 2 formulas:
      padded_so_far.pad_is_set         pad = 1, but max(minus(width, len(string)), 0) = 0
       --> leftpad.invariants.modality:3:3
      padded_so_far.length_so_far      len(result) = 1, but plus(len(string), added) = 2
       --> leftpad.invariants.modality:6:3
     --> leftpad.modality:12:3   q2 -> q2 does not leave /in/string.text alone
    = help: promise it on the edge: -modifies(/in/string.text), or a parent path
    = replay: modal run leftpad.modality --replay .modal/cex/1.toml
✓ rule      leftpad_spec.length    at q2 -> q3; no other edge can set /out/done.bool
✓ rule      leftpad_spec.padding   at q2 -> q3; no other edge can set /out/done.bool
✓ rule      leftpad_spec.suffix    at q2 -> q3; no other edge can set /out/done.bool
? term      q2 -> q2   unknown: no decreases measure, so the loop may never end
✓ progress  q2         the guards of q2 -> q2 and q2 -> q3 cover every state

7 of 9 proven, 1 refuted, 1 unknown                          exit 1
  = help: add `decreases q2 -> q2 by <term>` to the invariant file
not verified: 2 obligations are not proven
```

A step that may rewrite the input can make any spec about that input
false after the fact. The prover takes a missing promise at its word:
that move may write anything there. Put it back.

## 5. Does it end?

Look at the rules again: "no move may mark the output done unless the
property holds." A loop that never ends never marks anything done, so it
meets all three rules. The rules promise that **any** answer is right.
They do not promise that there **is** an answer. That is what `term` and
`progress` add.

To show a loop ends, give a **measure**: a number that is never below 0
while the loop can step, and that drops every step. Here it is how much
padding is left. Add this line to the end of
`leftpad.invariants.modality`, after the invariant:

```modality
decreases q2 -> q2 by minus(/out/pad.natural, /out/added.natural)
```

Like the invariant, it is a claim about this model, and it is never
posted.

```
$ modal prove leftpad.modality leftpad.rules.modality leftpad.invariants.modality
leftpad.modality · leftpad.rules.modality (3 formulas) · leftpad.invariants.modality (6 formulas)
semantics s1 · prover 0.1 · state: any
✓ init      q1 -> q2   meets invariant padded_so_far
✓ type      q1 -> q2   max(minus(width, len(string)), 0) fits /out/pad.natural
✓ type      q2 -> q2   plus(added, 1) fits /out/added.natural
✓ consec    q2 -> q2   keeps invariant padded_so_far
✓ rule      leftpad_spec.length    at q2 -> q3; no other edge can set /out/done.bool
✓ rule      leftpad_spec.padding   at q2 -> q3; no other edge can set /out/done.bool
✓ rule      leftpad_spec.suffix    at q2 -> q3; no other edge can set /out/done.bool
✓ term      q2 -> q2   by minus(pad, added): at least 0 under the guard, drops by at least 1 each step
✓ progress  q2         the guards of q2 -> q2 and q2 -> q3 cover every state

9 of 9 proven                                                exit 0
verified · leftpad meets leftpad_spec for every input · the loop at q2 always ends
  semantics s1 · prover 0.1 · every certificate checked again on its own
```

A number that is at least 0 and drops by at least 1 every step can only
drop so many times: here, at most `pad` times. And `progress` says the
loop never gets stuck at `q2` with neither edge allowed.

`progress` is checked at `q2` only. A run can still stop at `q1` when
the start edge refuses its inputs, such as `fill = "ab"`. That is the
spec's precondition at work, not a stuck loop.

## 6. Explain a proof

A ✓ should be something you could check by hand. `--explain` prints
the steps:

```
$ modal prove leftpad.modality leftpad.rules.modality leftpad.invariants.modality --explain "rule leftpad_spec.length"
rule leftpad_spec.length at q2 -> q3
  have  padded_so_far.added_le_pad
  have  padded_so_far.ends_with_string
  have  padded_so_far.length_so_far
  have  padded_so_far.pad_is_set
  have  guard, leftpad.modality:17
  split on max(minus(width, len(string)), 0):
    case minus(width, len(string)) ≥ 0:
      split on max(width, len(string)):
        case width ≥ len(string):
          split on not (len(result) = width):
            case len(result) < width:
              ∎ arithmetic: 1·(pad = minus(width, len(string))) + 1·(len(result) = plus(len(string), added)) + 1·(added ≥ pad) + 1·(len(result) < width)
            case len(result) > width:
              ∎ arithmetic: 1·(pad = minus(width, len(string))) + 1·(len(result) = plus(len(string), added)) + 1·(added ≤ pad) + 1·(len(result) > width)
        case width < len(string):
          split on not (len(result) = len(string)):
            case len(result) < len(string):
              ∎ arithmetic: 1·(minus(width, len(string)) ≥ 0) + 1·(width < len(string))
            case len(result) > len(string):
              ∎ arithmetic: 1·(minus(width, len(string)) ≥ 0) + 1·(width < len(string))
    case minus(width, len(string)) < 0:
      split on max(width, len(string)):
        case width ≥ len(string):
          split on not (len(result) = width):
            case len(result) < width:
              ∎ arithmetic: 1·(minus(width, len(string)) < 0) + 1·(width ≥ len(string))
            case len(result) > width:
              ∎ arithmetic: 1·(minus(width, len(string)) < 0) + 1·(width ≥ len(string))
        case width < len(string):
          split on not (len(result) = len(string)):
            case len(result) < len(string):
              ∎ arithmetic: 1·(result ends with string) + 1·(len(result) < len(string))
            case len(result) > len(string):
              ∎ arithmetic: 1·(pad = 0) + 1·(len(result) = plus(len(string), added)) + 1·(added ≤ pad) + 1·(len(result) > len(string))
  ∎  8 cases
  used 4 of 6 invariant formulas: added_le_pad, ends_with_string, length_so_far, pad_is_set
```

Each `have` names where a fact came from. The proof splits on each
`max` and on each way an equation can fail, and closes every case with
arithmetic: the multipliers show which facts add up to `0 < 0`. A step
that rests on a fact about text names the lemma it used, such as
`ends_with_concat` or `prefix_repeats_cons`.

`--explain` with no obligation prints which formulas each obligation
used:

```
$ modal prove leftpad.modality leftpad.rules.modality leftpad.invariants.modality --explain
invariant padded_so_far, formula by formula:
  pad_is_set         used by  consec, rule.length, rule.padding
  length_so_far      used by  consec, rule.length, rule.padding
  one_char_fill      used by  consec
  ends_with_string   used by  consec, rule.length, rule.padding, rule.suffix
  padded_with_fill   used by  consec, rule.padding
  added_le_pad       used by  rule.length, rule.padding, term
every formula is used; none can be cut
```

That is the cut from section 3, answered in advance: `one_char_fill` is
used by `consec`, so it cannot go.

## 7. Let the prover sort your guesses

You wrote the invariant by hand. The prover can also start from
guesses. `--seed` takes your search domain and keeps the guesses that
were true at every `q2` state the search reached. `--houdini` then drops
any guess that is not established or not kept, over and over, until
what is left holds together. Last, it cuts what the rules do not need:

```
$ modal prove leftpad.modality leftpad.rules.modality --houdini "q2 -> *" --seed domains.toml
on q2 -> * · 13 candidates true at every q2 state in 225 bounded runs
  round 1
    dropped  width_le_5                         not established: q1 -> q2 allows otherwise
    dropped  pad_le_5                           not established: q1 -> q2 allows otherwise
    dropped  len_string_le_3                    not established: q1 -> q2 allows otherwise
    dropped  len_result_le_5                    not established: q1 -> q2 allows otherwise
  round 2
    dropped  added_le_5                         not kept: the prover cannot show a step keeps it
  kept 8, inductive together
  ✓ strong enough for the rules; 6 of the 8 are needed:
    pad_is_set
    len_result_is_added_plus_len_string
    added_le_pad
    len_fill_is_1
    result_starts_with_added_fill
    result_ends_with_string
  = write: add --write to save them as invariant suggested_q2
```

The six it needs are the six formulas you wrote, under its own names.
The five it drops are worth a look. Each held on every run you
searched, and each is false for some input. That is the gap between a
search and a proof. `added_le_5` even survived round 1, because
`width_le_5` was still there to prop it up.

Houdini only removes guesses. If what is left is too weak, the fact you
need was never one of the guesses, and you write it yourself, as you
wrote `added_le_pad`. Every verdict comes from the same prover as
`modal prove`. `--write` adds what it found to the invariant file.

## 8. Check it again in Lean

A ✓ from `modal prove` rests on the prover being right. To check each
obligation a second time without trusting the prover, export it:

```
$ modal prove leftpad.modality leftpad.rules.modality leftpad.invariants.modality --emit-lean out
leftpad.modality · leftpad.rules.modality (3 formulas) · leftpad.invariants.modality (6 formulas)
semantics s1 · prover 0.1 · state: any
✓ init      q1 -> q2   meets invariant padded_so_far
✓ type      q1 -> q2   max(minus(width, len(string)), 0) fits /out/pad.natural
✓ type      q2 -> q2   plus(added, 1) fits /out/added.natural
✓ consec    q2 -> q2   keeps invariant padded_so_far
✓ rule      leftpad_spec.length    at q2 -> q3; no other edge can set /out/done.bool
✓ rule      leftpad_spec.padding   at q2 -> q3; no other edge can set /out/done.bool
✓ rule      leftpad_spec.suffix    at q2 -> q3; no other edge can set /out/done.bool
✓ term      q2 -> q2   by minus(pad, added): at least 0 under the guard, drops by at least 1 each step
✓ progress  q2         the guards of q2 -> q2 and q2 -> q3 cover every state

9 of 9 proven                                                exit 0
verified · leftpad meets leftpad_spec for every input · the loop at q2 always ends
  semantics s1 · prover 0.1 · every certificate checked again on its own
wrote out: a Lake project (leanprover/lean4:v4.14.0, no mathlib) with 18 obligation theorems
  check them: cd out && lake build
$ cd out && lake build
Build completed successfully.
```

Each theorem is one proven obligation, stated as: these facts together
are impossible. Here is the length rule at the finish:

```lean
theorem rule_leftpad_spec_length_at_q2_to_q3
    (fill : Text) (string : Text) (width : Int) (added : Nat) (pad : Nat) (result : Text)
    (h1 : (pad : Int) = (max (width - ((string).length : Int)) (0 : Int)))
    (h2 : ((result).length : Int) = (((string).length : Int) + (added : Int)))
    (h3 : ((fill).length : Int) = (1 : Int))
    (h4 : EndsWith result string)
    (h5 : PrefixRepeats result (added : Int) fill)
    (h6 : (added : Int) ≤ (pad : Int))
    (h7 : (added : Int) ≥ (pad : Int))
    (h8 : ¬ (((result).length : Int) = (max width ((string).length : Int)))) : False := by
  have := Modality.ends_len h4
  have := Modality.prefix_len h5
  try simp only [List.length_append, List.length_cons, List.length_nil] at *
  first | omega | contradiction | simp_all
```

Numbers are `Int` (a `.natural` is a `Nat`), text is a list of
characters, and the proof is `omega` after the text lemmas the
certificate used. The lemmas, such as `ends_len` and `prefix_cons`, are
proved in `ModalityProof/Text.lean`, in the same project. No mathlib,
and `lake build` needs neither `modal` nor the prover.

What Lean checks is each obligation. That the obligations together mean
the model meets its rules is how the prover works, and is not yet a Lean
theorem. `--emit-certificate DIR` writes the same proofs as JSON, for
other checkers.

## 9. Pin it in tests

Add the proof, and the mistakes from this tutorial, to
`leftpad.test.toml`:

```toml
[[run]]
name = "pads foo to five"
set = { "/in/fill.text" = "0", "/in/width.integer" = 5, "/in/string.text" = "foo" }
expect = { "/out/result.text" = "00foo", "/out/done.bool" = true }

[[run]]
name = "never truncates"
set = { "/in/fill.text" = "0", "/in/width.integer" = 1, "/in/string.text" = "foo" }
expect = { "/out/result.text" = "foo" }

[[check]]
name = "no counterexample on small inputs"
bounded = "domains.toml"
inductive = true
expect = { all = "no-counterexample" }

[[mutation]]
name = "the step must leave done alone"
edge = "q2 -> q2"
remove = "-modifies(/out/done.bool)"
bounded = "domains.toml"
expect = { "leftpad_spec.length" = "violated" }

[[mutation]]
name = "width must be whole"
retype = { "/in/width.integer" = "/in/width.num" }
expect = { "write /out/pad.natural" = "undecided" }

[[mutation]]
name = "fill must be one character"
edge = "q1 -> q2"
remove = "+num_eq(len(/in/fill.text), 1)"
bounded = "domains.toml"
expect = { "leftpad_spec.padding" = "violated" }

[[prove]]
name = "leftpad meets its spec for every input"
expect = { all = "proven" }

[[mutation]]
name = "the invariant must say fill is one character"
invariant = "padded_so_far"
remove = "one_char_fill"
prove = true
expect = { "consec q2 -> q2" = "refuted" }

[[mutation]]
name = "the step must leave the inputs alone"
edge = "q2 -> q2"
remove = "-modifies(/in)"
prove = true
expect = { "consec q2 -> q2" = "refuted" }

[[mutation]]
name = "the loop needs a measure"
remove = "decreases q2 -> q2"
prove = true
expect = { "term q2 -> q2" = "unknown" }

[[mutation]]
name = "width must be whole, for the prover too"
retype = { "/in/width.integer" = "/in/width.num" }
prove = true
expect = { "type q1 -> q2" = "refuted" }
```

```
$ modal test
leftpad.test.toml
  ✓ pads foo to five
  ✓ never truncates
  ✓ no counterexample on small inputs                (225 runs)
  ✓ the step must leave done alone                   (violated, as expected)
  ✓ width must be whole                              (undecided, as expected)
  ✓ fill must be one character                       (violated, as expected)
  ✓ leftpad meets its spec for every input           (9 of 9 proven)
  ✓ the invariant must say fill is one character     (refuted, as expected)
  ✓ the step must leave the inputs alone             (refuted, as expected)
  ✓ the loop needs a measure                         (unknown, as expected)
  ✓ width must be whole, for the prover too          (refuted, as expected)
11 passed                                                    exit 0
```

The last mutation is Bug 2 from the last tutorial, seen by the prover:
with `width = 1/2`, the start edge would write `1/2` to a `.natural`.
`modal check` called it undecided. The prover refutes it with that
input.

Verdicts are the same on every machine. The prover's budgets count
steps, not seconds, so CI never flips a verdict because a runner was
slow.

## 10. Faster loops

- `modal prove --watch …` proves again on every save.
- `modal prove --only rule …` shows one kind of obligation.

The editor extension does not show obligations yet.

## What you have shown, and what you have not

- **Proved:** for every `fill`, `width` and `string` the start edge
  accepts (one-character `fill`, whole `width`), every run that marks
  the output done has a correct result. The loop at `q2` always ends,
  and never gets stuck.
- **Checked twice:** each obligation by `modal prove`, with its
  certificate checked again, and by Lean's kernel on the exported
  theorems.
- **Rests on:** the semantics `s1`, the same one `modal run` and the
  search use, and the prover's method: that these obligations together
  mean the model meets its rules.
- **Not promised to anyone:** `padded_so_far` and the `decreases`
  measure. They are how you proved the rules, not part of them. A
  contract refuses them, and a later model may need different ones.
- **Length** is counted in characters (Unicode scalar values), as in
  the last tutorial.
- **Not shown:** anything about a contract, a network or a machine
  running this model. The proof is about the model. It also does not
  say that someone will post inputs, or that a run will keep moving: it
  says what is true of every run that does.
