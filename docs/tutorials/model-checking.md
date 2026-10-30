---
sidebar_position: 19
title: Model Checking with Modality
---

# Model Checking with Modality

You will write a small model and a rule, check them, read why a check
fails, and fix it. Then you will write leftpad as a model, run it,
search every small input for a bug, and pin what you found in tests.

You need `modal`. You do not need a key, a contract, or a network.

These commands are on `main` and not yet in a release binary. Until
they are, build `modal`
[from source](/docs/getting-started/installation#install-from-source).
The test `rust/modality/tests/walkthrough/model-checking.walk` runs every
`modal` command on this page and checks that it prints what the page
shows.

## 1. A check in thirty seconds

A **model** is the moves something can make: states, and edges between
them. An edge's labels say what must be true of a move. A **rule** says
something about every move the model allows.

Save this as `approvals.modality`:

```modality
model approvals {
  initial q0

  q0 -> q1 [+signed_by(/parties/alice.id)]
  q1 -> q1 [+signed_by(/parties/alice.id) +signed_by(/parties/bob.id)]
}

formula alice_signs_every_move {
  always([-signed_by(/parties/alice.id)] false)
}
```

The formula reads: always, after any move *without* Alice's signature,
`false`. Nothing can be false, so no such move may exist.

```
$ modal check approvals.modality
approvals.modality · model approvals · theory v5 · state: any
✓ alice_signs_every_move                                       holds

1 formula holds                                              exit 0
```

Now give Bob a move of his own. Add this edge:

```modality
  q1 -> q2 [+signed_by(/parties/bob.id)]
```

```
$ modal check approvals.modality
approvals.modality · model approvals · theory v5 · state: any
✗ alice_signs_every_move                                    violated

error[check/box-violated]: a move the formula forbids is possible
 --> approvals.modality:6:3
  |
6 |   q1 -> q2 [+signed_by(/parties/bob.id)]
  |   ^^^^^^^^ this edge does not require signed_by(/parties/alice.id)
  |
  = path: q0 -> q1 -> q2
  = breaks: always([-signed_by(/parties/alice.id)] false)
  = help: add +signed_by(/parties/alice.id) to the edge, or remove it
  = docs: modal explain check/box-violated

1 formula violated                                           exit 1
```

Every failure has the same parts: the edge, the path that reaches it,
the formula it breaks, and a fix. `modal explain check/box-violated`
says more about this one.

Fix the edge (`[+signed_by(/parties/alice.id) +signed_by(/parties/bob.id)]`)
and the check passes again.

## 2. Must and may

That box (`[...]`) is a promise about **every** matching move. A
diamond (`<...>`) promises that **some** matching move exists. Add a
second formula that says Bob can still sign something:

```modality
formula bob_can_sign {
  eventually(<+signed_by(/parties/bob.id)> true)
}
```

```
$ modal check approvals.modality
approvals.modality · model approvals · theory v5 · state: any
✓ alice_signs_every_move                                       holds
✓ bob_can_sign                                                 holds

2 formulas hold                                              exit 0
```

Delete both of Bob's edges and check again:

```
$ modal check approvals.modality
approvals.modality · model approvals · theory v5 · state: any
✓ alice_signs_every_move                                       holds
✗ bob_can_sign                                              violated

error[check/diamond-unreachable]: no reachable move carries signed_by(/parties/bob.id)
 --> approvals.modality:12:3
   |
12 |   eventually(<+signed_by(/parties/bob.id)> true)
   |              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ wanted somewhere on a path
   |
  = reachable: q0, q1
  = moves from them: q0 -> q1 [+signed_by(/parties/alice.id)]
  = help: add an edge that carries the label, or drop the formula
  = docs: modal explain check/diamond-unreachable

1 formula violated                                           exit 1
```

"Eventually" means a path can reach it, not that a later move will
happen. No rule can make someone act.

## 3. Rules in their own files

Contracts keep rules apart from models. `modal check` takes both, as
files or as directories of them. With the model in `model/` and both
formulas in `rules/signing.modality`:

```modality
rule signing {
  formula alice_signs_every_move {
    always([-signed_by(/parties/alice.id)] false)
  }
  formula bob_can_sign {
    eventually(<+signed_by(/parties/bob.id)> true)
  }
}
```

```
$ modal check model/approvals.modality rules/
model/approvals.modality · rules/signing.modality (2 formulas) · theory v5 · state: any
✓ signing.alice_signs_every_move                               holds
✓ signing.bob_can_sign                                         holds

2 formulas hold                                              exit 0
```

The header always says which theory decided the labels and which
starting state was assumed.

**Starting state.** `state: any` means no posted data is assumed, so
"holds" means holds whatever has been posted. A check from particular
data can come out differently. Here is a model where every move
promises not to touch `/parties`:

```modality
model frozen {
  initial q0
  q0 -> q1 [+signed_by(/parties/alice.id) -modifies(/parties)]
  q1 -> q2 [+signed_by(/parties/bob.id) -modifies(/parties)]
}

formula alice_signs_every_move {
  always([-signed_by(/parties/alice.id)] false)
}
```

From any state, Bob's edge breaks the rule:

```
$ modal check frozen.modality
frozen.modality · model frozen · theory v5 · state: any
✗ alice_signs_every_move                                    violated

error[check/box-violated]: a move the formula forbids is possible
 --> frozen.modality:4:3
  |
4 |   q1 -> q2 [+signed_by(/parties/bob.id) -modifies(/parties)]
  |   ^^^^^^^^ this edge does not require signed_by(/parties/alice.id)
  |
  = path: q0 -> q1 -> q2
  = breaks: always([-signed_by(/parties/alice.id)] false)
  = help: add +signed_by(/parties/alice.id) to the edge, or remove it
  = docs: modal explain check/box-violated

1 formula violated                                           exit 1
```

But if Bob's id was never posted, and no move may post it, nobody can
ever take that edge. Say what has been posted in a TOML file,
`only-alice.toml`:

```toml
"/parties/alice.id" = "alice-key"
```

```
$ modal check frozen.modality --state only-alice.toml
frozen.modality · model frozen · theory v5 · state: only-alice.toml
✓ alice_signs_every_move                                       holds
  used: /parties/bob.id is not posted; q1 -> q2 can never be taken

1 formula holds                                              exit 0
```

That is the verdict a contract holding that data would give when the
rule is added.

## 4. Lint: what parses but does not say what it seems to

```modality
formula bob_signed {
  always([+signed_by(/parties/bob.id)] true)
}
```

```
$ modal check approvals.modality
warning[lint/vacuous-box-guard]: this box holds everywhere
 --> approvals.modality:13:10
   |
13 |   always([+signed_by(/parties/bob.id)] true)
   |          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ a box of `true` is true even with no such move
   |
   = help: to say the move is possible, write <...> true instead of [...] true

approvals.modality · model approvals · theory v5 · state: any
✓ alice_signs_every_move                                       holds
✓ bob_signed                                                   holds

2 formulas hold                                              exit 0
```

## 5. Leftpad as a model

[Leftpad](https://github.com/hwayne/lets-prove-leftpad) pads a string
on the left. `leftpad("0", 5, "foo")` is `"00foo"`, and
`leftpad("0", 1, "foo")` is `"foo"`. The challenge calls the inputs
`c`, `n` and `s`; here they are `fill`, `width` and `string`. Here it is
as a model: post the inputs, then put one `fill` in front of `result`
per move until it is long enough. Save it as `leftpad.modality`:

```modality
model leftpad {
  initial q0

  // post the inputs: any fill, width and string
  q0 -> q1 [-modifies(/out)]
  // start: result := string, added := 0, pad := max(width - len(string), 0)
  q1 -> q2 [+sets_from(/out/result.text, /in/string.text) +sets(/out/added.natural, 0)
            +sets(/out/pad.natural, max(minus(/in/width.num, len(/in/string.text)), 0))
            -modifies(/in) -modifies(/out/done.bool)]
  // step: while added < pad, put fill in front of result
  q2 -> q2 [+num_lt(/out/added.natural, /out/pad.natural)
            +sets(/out/result.text, concat(/in/fill.text, /out/result.text))
            +sets(/out/added.natural, plus(/out/added.natural, 1))
            -modifies(/in) -modifies(/out/pad.natural)]
  // finish
  q2 -> q3 [+num_gte(/out/added.natural, /out/pad.natural) +sets(/out/done.bool, true)
            -modifies(/in) -modifies(/out/result.text)]
}
```

A label reads the state a move starts from. `sets_from(/p, /q)` copies
`/q` into `/p`; `sets(/p, v)` writes a value: a literal, or a term such
as `plus(/out/added.natural, 1)`, worked out before the move.
`-modifies(/in)` says a move leaves the inputs alone. A move may write
anything it does not promise to leave alone. That will matter.

Numbers come in three types: `.natural` (0, 1, 2, …), `.integer`
(…, −1, 0, 1, …) and `.rational` (any exact fraction; `.num` is another
name for it). `added` and `pad` count characters, so they are
`.natural`. `width` is just "a number" for now. A move that writes a
value its path cannot hold is refused.

Run it:

```
$ modal run leftpad.modality --set /in/fill.text=0 --set /in/width.num=5 --set /in/string.text=foo
q0 -> q1 -> q2 -> q2 -> q2 -> q3
/out/result.text = "00foo"   /out/added.natural = 2   /out/pad.natural = 2   /out/done.bool = true

$ modal run leftpad.modality --set /in/fill.text=0 --set /in/width.num=1 --set /in/string.text=foo
q0 -> q1 -> q2 -> q3
/out/result.text = "foo"   /out/added.natural = 0   /out/pad.natural = 0   /out/done.bool = true
```

`--trace` prints every move's writes:

```
$ modal run leftpad.modality --set /in/fill.text=0 --set /in/width.num=1 --set /in/string.text=foo --trace
q0 -> q1   /in/fill.text = "0"  /in/string.text = "foo"  /in/width.num = 1
q1 -> q2   /out/result.text = "foo"  /out/added.natural = 0  /out/pad.natural = 0
q2 -> q3   /out/done.bool = true
q0 -> q1 -> q2 -> q3
/out/result.text = "foo"   /out/added.natural = 0   /out/pad.natural = 0   /out/done.bool = true
```

## 6. The spec as a rule

The challenge's three properties, word for word. They mention only the
inputs and the output: nothing about `added`, `pad`, or how the loop
works. Save this as `leftpad.rules.modality`:

```modality
rule leftpad_spec {
  formula length {
    always([+sets(/out/done.bool, true)
            -num_eq(len(/out/result.text), max(/in/width.num, len(/in/string.text)))] false)
  }
  formula padding {
    always([+sets(/out/done.bool, true)
            -text_prefix_repeats(/out/result.text,
               minus(len(/out/result.text), len(/in/string.text)), /in/fill.text)] false)
  }
  formula suffix {
    always([+sets(/out/done.bool, true)
            -text_ends_with(/out/result.text, value(/in/string.text))] false)
  }
}
```

Each one reads: no move may mark the output done unless the property
holds. In `text_ends_with`, a plain second argument is literal text, as
on a network; `value(/in/string.text)` reads the path.

```
$ modal check leftpad.modality leftpad.rules.modality
leftpad.modality · leftpad.rules.modality (3 formulas) · theory v5 · state: any
? leftpad_spec.length                                      undecided
  2 edges may break it, depending on data the labels do not show:
    leftpad.modality:11  q2 -> q2  may set /out/done.bool (not left alone); num_eq(len(result), …) is open
    leftpad.modality:16  q2 -> q3  sets /out/done.bool; num_eq(len(result), …) is open
? leftpad_spec.padding                                     undecided   (same 2 edges)
? leftpad_spec.suffix                                      undecided   (same 2 edges)
? write to /out/pad.natural                                undecided
    leftpad.modality:7  q1 -> q2  max(minus(width, len(string)), 0) is a rational ≥ 0, and a .natural must also be whole

3 formulas and 1 write undecided                             exit 2
  = help: search small inputs with --bounded, or prove for all inputs with modal prove
```

`?` is not a pass. The check looked at labels and types only. Whether
`len(result)` is right at the finish depends on what the loop wrote,
and whether `pad` is whole depends on `width`. To decide them, run the
model on data.

Terms such as `len(…)` are for checking and proving on your machine. A
network refuses a model or rule that has one; no network theory reads
them yet.

## 7. Search every small input

List the inputs to try in `domains.toml`. Put in values you think are
bad, too: the search only tries what you give it.

```toml
depth = 12

[domain]
"/in/fill.text"   = ["0", "é", "ab"]
"/in/width.num"   = [-1, 0, 1, 3, 5, "1/2"]
"/in/string.text" = { alphabet = ["a", "b"], max_len = 3 }
# loop state, used by --inductive (section 8)
"/out/result.text"   = { alphabet = ["0", "é", "a", "b"], max_len = 4 }
"/out/added.natural" = [0, 1, 2, 3]
"/out/pad.natural"   = [0, 1, 2, 3]

[havoc]                 # values a move may write where it promised nothing
"/out/done.bool" = [true, false]
```

```
$ modal check leftpad.modality leftpad.rules.modality --bounded domains.toml
bounded search · 3 × 6 × 15 inputs · depth 12 · 2,136 runs
✗ leftpad_spec.length                                       violated
  fill = "0", width = 1, string = ""
  q0 -> q1 -> q2 -> q2
              ^^^^^^^^ this move also wrote /out/done.bool = true while added = 0 < pad = 1
  len(result) = 0, but max(width, len(string)) = 1
  --> leftpad.modality:11:3   q2 -> q2 does not leave /out/done.bool alone
  = replay: modal run leftpad.modality --replay .modal/cex/1.toml

✗ leftpad_spec.padding                                      violated
  fill = "ab", width = 1, string = ""
  q0 -> q1 -> q2 -> q2 -> q3
                    ^^^^^^^^ this move breaks it
  the first 2 characters of "ab" are not each "ab"
  --> leftpad.modality:16:3   q2 -> q3
  = replay: modal run leftpad.modality --replay .modal/cex/2.toml

✗ write to /out/pad.natural                                 violated
  fill = "0", width = 1/2, string = ""
  q0 -> q1 -> q2
        ^^^^^^^^ would write 1/2 to /out/pad.natural, so this move can never be made
  --> leftpad.modality:7:3   the value comes from /in/width.num, a rational
  = replay: modal run leftpad.modality --replay .modal/cex/3.toml

✓ leftpad_spec.suffix                no counterexample in 2,136 runs

2 formulas and 1 write violated (this is a search of the domain, not a proof)  exit 1
```

**Bug 1: the step can mark the output done.** It never promised to
leave `/out/done.bool` alone, so a step may write it. Replay the
counterexample to see the run:

```
$ modal run leftpad.modality --replay .modal/cex/1.toml
q0 -> q1   /in/fill.text = "0"  /in/string.text = ""  /in/width.num = 1
q1 -> q2   /out/result.text = ""  /out/added.natural = 0  /out/pad.natural = 1
q2 -> q2   /out/result.text = "0"  /out/added.natural = 1  /out/done.bool = true
q0 -> q1 -> q2 -> q2
```

Add `-modifies(/out/done.bool)` to the step edge.

**Bug 2: `width` can be a fraction.** `/in/width.num` holds any exact
rational, so `width = 1/2` is a real input, and then `pad` would be
`1/2`, which a `.natural` cannot hold. No implementation could meet the
spec there anyway: it asks for a string of length `1/2`. A whole
`width` is part of the spec, and the way to say so is the type. Rename
`/in/width.num` to `/in/width.integer` in both files. Use `.integer`,
not `.natural`: leftpad allows a negative `width`, which just means no
padding. The plain check now settles the write by itself, because the
largest of an integer minus a natural, and 0, is a natural:

```
$ modal check leftpad.modality leftpad.rules.modality
leftpad.modality · leftpad.rules.modality (3 formulas) · theory v5 · state: any
? leftpad_spec.length                                      undecided
  1 edge may break it, depending on data the labels do not show:
    leftpad.modality:16  q2 -> q3  sets /out/done.bool; num_eq(len(result), …) is open
? leftpad_spec.padding                                     undecided   (same 1 edge)
? leftpad_spec.suffix                                      undecided   (same 1 edge)

3 formulas undecided                                         exit 2
  = help: search small inputs with --bounded, or prove for all inputs with modal prove
```

The domain file still lists `"1/2"` for `width`, and the search says so:

```
$ modal check leftpad.modality leftpad.rules.modality --bounded domains.toml
error[domain/type]: "1/2" is not an .integer
 --> domains.toml:5:40
  = help: the type already rules this input out; remove it
```

Remove it, and search again:

```
$ modal check leftpad.modality leftpad.rules.modality --bounded domains.toml
bounded search · 3 × 5 × 15 inputs · depth 12 · 225 runs
✗ leftpad_spec.length                                       violated
  fill = "ab", width = 1, string = ""
  q0 -> q1 -> q2 -> q2 -> q3
                    ^^^^^^^^ this move breaks it
  len(result) = 2, but max(width, len(string)) = 1
  --> leftpad.modality:16:3   q2 -> q3
  = replay: modal run leftpad.modality --replay .modal/cex/1.toml

✗ leftpad_spec.padding                                      violated
  fill = "ab", width = 1, string = ""
  q0 -> q1 -> q2 -> q2 -> q3
                    ^^^^^^^^ this move breaks it
  the first 2 characters of "ab" are not each "ab"
  --> leftpad.modality:16:3   q2 -> q3
  = replay: modal run leftpad.modality --replay .modal/cex/2.toml

✓ leftpad_spec.suffix                  no counterexample in 225 runs

2 formulas violated (this is a search of the domain, not a proof)  exit 1
```

**Bug 3: `fill` can be more than one character.** With `fill = "ab"`
the result is two characters too long, and it does not start with
copies of one character. The first search found the length failure
through Bug 1 first; with that fixed, `"ab"` breaks it too. Add
`+num_eq(len(/in/fill.text), 1)` to the start edge:

```modality
  q1 -> q2 [+num_eq(len(/in/fill.text), 1)
            +sets_from(/out/result.text, /in/string.text) +sets(/out/added.natural, 0)
```

```
$ modal check leftpad.modality leftpad.rules.modality --bounded domains.toml
bounded search · 3 × 5 × 15 inputs · depth 12 · 225 runs
✓ leftpad_spec.length                  no counterexample in 225 runs
✓ leftpad_spec.padding                 no counterexample in 225 runs
✓ leftpad_spec.suffix                  no counterexample in 225 runs

no counterexample (this is a search of the domain, not a proof)  exit 0
```

Fewer runs now: the start edge refuses `fill = "ab"`, and `width = 1/2`
is no longer an input. Length is counted in characters (`"é"` is one),
not bytes.

## 8. Find the loop's invariant

A search tries some inputs. To be sure of *every* input, you will need
an **invariant**: a fact about the loop that is true when the loop
starts and stays true after each step. Model checking helps you find
one before you try to prove anything.

Write a first guess in a new file, `leftpad.invariants.modality`:

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
}
```

An invariant is written like a rule, with two differences:

- **`on q2 -> *`**: its boxes see only the moves out of `q2`. A label
  reads the state a move starts from, so each formula reads "no move
  leaves `q2` where this is false": the fact holds at `q2`. A rule's
  boxes see every move.
- **It is never posted.** It talks about `added` and `pad`, which are
  how this model works, not what it promises. Rules mention only the
  inputs and the output, and a contract keeps them forever. An
  invariant is a claim about this model, and a contract refuses one.

Each fact gets its own formula: `[-a -b] false` would only refuse a move
where both fail.

`--inductive` draws states from the domain's entries, not only from
states a run reaches, and asks three questions: does it hold at every
`q2` state a run reaches, does every step keep it, and is it strong
enough for the rules?

```
$ modal check leftpad.modality leftpad.rules.modality leftpad.invariants.modality --bounded domains.toml --inductive
bounded search · 3 × 5 × 15 inputs · depth 12 · 225 runs
✓ leftpad_spec.length                  no counterexample in 225 runs
✓ leftpad_spec.padding                 no counterexample in 225 runs
✓ leftpad_spec.suffix                  no counterexample in 225 runs

invariant padded_so_far (on q2 -> *)
  ✓ holds at every q2 state a run reaches (256 states)
  ✓ every step keeps it
  ✗ too weak for leftpad_spec.length at q2 -> q3
    a state the invariant allows, where q2 -> q3 breaks the rule:
      fill = "0", width = -1, string = "", result = "0", added = 1, pad = 0
      padded_so_far holds and added ≥ pad; len(result) = 1, but max(width, len(string)) = 0
    no run reaches this state. The invariant is missing a fact that rules it out.

the invariant fails on this domain (this is a search of the domain, not a proof)  exit 1
```

The loop never makes `added` larger than `pad`, but the invariant does
not say so. Add a sixth formula to `padded_so_far`, after
`padded_with_fill`. The rules do not change: this is a fact about how
the loop works, not part of what leftpad promises.

```modality
  formula added_le_pad {
    always([-num_lte(/out/added.natural, /out/pad.natural)] false)
  }
```

(It never had to say `added ≥ 0`: a `.natural` cannot be negative.)

```
$ modal check leftpad.modality leftpad.rules.modality leftpad.invariants.modality --bounded domains.toml --inductive
bounded search · 3 × 5 × 15 inputs · depth 12 · 225 runs
✓ leftpad_spec.length                  no counterexample in 225 runs
✓ leftpad_spec.padding                 no counterexample in 225 runs
✓ leftpad_spec.suffix                  no counterexample in 225 runs

invariant padded_so_far (on q2 -> *)
  ✓ holds at every q2 state a run reaches (256 states)
  ✓ every step keeps it
  ✓ strong enough for leftpad_spec.length, .padding, .suffix at q2 -> q2, q2 -> q3
on this domain. To show it for every input: modal prove

no counterexample (this is a search of the domain, not a proof)  exit 0
```

## 9. Pin it in tests

Save the runs and the bugs you found as `leftpad.test.toml`. A test
file reads the model, rules and invariants next to it with the same
name (`leftpad.modality`, `leftpad.rules.modality`, …):

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
6 passed                                                     exit 0
```

A mutation test is how you know the rule is not too weak: break the
model on purpose, and the rule must notice. `modal test` exits non-zero
on any failure, so it runs in CI as it is.

## 10. Faster loops

- `modal check --watch …` checks again on every save.
- `modal explain <code>` says what a diagnostic means and what to do.
- **REPL.** `modal repl leftpad.modality` steps through a run:

  ```
  model leftpad at q0; :help lists commands
  > :set /in/fill.text=0 /in/width.integer=3 /in/string.text=ab
  > :step
  q0 -> q1   fill = "0"  string = "ab"  width = 3
  > :live
  q1 -> q2 [+num_eq(len(/in/fill.text), 1) +sets_from(/out/result.text, /in/string.text) +sets(/out/added.natural, 0) +sets(/out/pad.natural, max(minus(/in/width.integer, len(/in/string.text)), 0)) -modifies(/in) -modifies(/out/done.bool)]
  > :step
  q1 -> q2   result = "ab"  added = 0  pad = 1
  > :run
  q2 -> q2   result = "0ab"  added = 1
  q2 -> q3   done = true
  no move from q3 is possible
  ```

The editor extension and the in-browser playground do not run these
checks yet.

## What you have shown, and what you have not

- `modal check`: the model meets its rules **as far as labels can
  tell**, from the starting state in the header. ✓ is a fact about this
  model; ? means data decides.
- `--bounded`: no input **in your domain** breaks a rule, up to the
  depth. Three real bugs fell out of it. It is still a search, and it
  says so.
- `--inductive`: your invariant holds, is kept, and is strong enough,
  **on the domain**.
- Not shown: that leftpad is right for **every** `fill`, `width` and
  `string`. That is a proof. The next tutorial,
  [Leftpad, proved](/docs/tutorials/leftpad-proved), gives the same
  files and this invariant to `modal prove`.
