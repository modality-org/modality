# Predicate theory: Lean 4 spec and proof

A model's transitions carry labels such as `+num_gte(/escrow/paid.num,"100")`.
Commits are checked label by label. The **predicate theory** asks a
question label matching does not: *can these labels hold together at
all?* An edge whose labels cannot (a **dead edge**) is a false promise
in the model. The theory also answers *does one label set imply another?*
(`paid ≥ 120` implies `paid ≥ 100`), so a rule can be met by an edge that
names a stricter bound.

This directory is the specification of theory `V1`, in Lean 4. It
covers every sort the Rust decides: numbers, booleans, presence, text,
signer sets under a prefix, the paths a commit writes and posts, and
static labels. It defines what the labels mean, says how a predicate
becomes constraints, and gives a checker. It proves that a `dead` verdict
is right and that a `live` verdict comes with a commit that takes the
edge. The Rust in `rust/modality-lang/src/theory/` is a transliteration.
A harness compares the two on random label sets, and a governance test
feeds the Rust witnesses to the evaluator.

## The example

Bob buys Alice's laptop for 100. The escrow model has two ways out of
`open`:

```modality
open --> released: +signed_by(/parties/alice.id) +num_gte(/escrow/paid.num,"100")
open --> refunded: +signed_by(/parties/bob.id)   +num_lt(/escrow/paid.num,"100")
                                                 +num_gte(/escrow/paid.num,"100")
```

The refund line was copied from the release line, and `num_gte` was not
deleted. Bob reads "I can take a refund while underpaid" and signs. No
commit can ever take that edge: `paid` cannot be below 100 and at least
100 at once.

- Theory version `V0` (today's behaviour) accepts the model.
- Theory version `V1` refuses it and names the edge and the two
  literals:

```text
Model has transitions no commit can take (predicate theory V1): flow: open --> refunded
[...] cannot hold together: +(< /escrow/paid.num 100), +(<= 100 /escrow/paid.num)
```

Lean proves the same edge dead in `Cases.lean`.

## Layout

| Path | Job |
|------|-----|
| `cases.json` | Grounding cases: labels, optional declarations and state, and the verdicts V1 must reach |
| `lean/PredicateTheory/Num.lean` | Exact rationals and their order laws |
| `lean/PredicateTheory/Fragment.lean` | What the labels mean: a world is accepted state, the keys that signed, and the commit's actions |
| `lean/PredicateTheory/Elab.lean` | How a predicate becomes constraints: declarations, the standard table, `expand` |
| `lean/PredicateTheory/Decide.lean` | The checker `dead` (eight per-sort checks) and `entails` |
| `lean/PredicateTheory/Witness.lean` | `live`: build a world from the labels and check it |
| `lean/PredicateTheory/Sound.lean` | `dead_sound`, `entails_sound`, `live_sound` |
| `lean/PredicateTheory/Runtime.lean` | The same with accepted state known: `deadIn_sound`, `liveIn_sound` |
| `lean/PredicateTheory/Vars.lean` | Edges with variables (`$k`) and holes (`!$k`): what taking one means, and `takesB`, which decides it from the commit's own paths |
| `lean/PredicateTheory/Flow.lean` | State flow along a model: which facts survive an edge, facts closed over a model, and edges no run takes after a step |
| `lean/PredicateTheory/Spec.lean` | The verdicts on labels as written, and what each one means |
| `lean/PredicateTheory/Cases.lean` | The escrow, decided by the proven checker; a flawed checker refuted |
| `lean/PredicateTheory/Generated.lean` | Every case in `cases.json`, as a theorem or checked example; written by `gen_cases.py` |
| `lean/Main.lean` | `pt-check`: the checker as a program, for the harness |
| `lean/agree.sh` | Builds `pt-check` and runs the Rust-vs-Lean harness |

## Build

```bash
cd experiments/predicate-theory/lean
lake build            # the proofs, every case, and pt-check (about 20 s)
python3 gen_cases.py  # after editing cases.json
```

Lean 4.14, no Mathlib. CI (`.github/workflows/predicate-theory.yml`)
checks that `Generated.lean` is current, builds everything, fails on
any `sorry` or `native_decide`, and runs the harness.

## How it works

**Meaning.** A world is the accepted state (a typed value, or nothing,
at each path), the keys that signed the commit, and the commit's
actions. Each constraint is a function of the world, written the way the
evaluator reads it. A numeric comparison on a path that holds no number
is false. `signed(p)` holds when `p` holds a key that signed. `card(q) ≥
n` counts distinct signed keys posted at `.id` paths under `q`.
`writes(p)` and `posts(p)` read the actions, and a static label such as
`+POST` holds when some action has that method. A predicate with no
declaration is opaque: it means whatever a parameter says, and no proof
may assume anything about it.

**Checker.** `dead` runs eight checks: a literal and its negation;
presence; booleans; the order of numbers, closed under composition;
text classes, joined through shared literals, with substring tests
checked against what the class must contain; a lower bound on the signed
keys under each prefix; the path lattice of writes and posts; and one
path forced to hold two types. Each check is the twin of a Rust
procedure.

**Witnesses.** `live` builds a world and evaluates every label on it
with the same meaning function the soundness proof uses. Numbers are
short decimals placed between the constants, so every order constraint
holds and no two unrelated values meet. Strings are the class literal,
or the required prefix, infixes, and suffix separated by a fresh
character. Keys are posted and signed where a signer label needs them.
When the construction fails, the verdict is `unknown`, never `live`.

**Known state.** At runtime the accepted state is known and the commit
is not. What the state says about each path becomes literals, and the
posted keys become a closed set. `live` then builds only the commit.

**Variables.** A path segment `$k.id` is a variable `k` (a name with no
dot) followed by a suffix. A hole `!$k.id` is every segment whose stem is
not `k`'s, followed by the suffix. A commit takes an edge when some names
for its variables make every label hold, each label with a hole for every
value of the hole. Labels only compare a label path with a world path, so
a name that is not a prefix of any segment in the world behaves like any
other such name. `takesB` therefore tries the prefixes of the world's
segments plus one fresh name, and skips the fresh name in a hole's
excluded slot.

**State flow.** A commit changes accepted state only at the paths its
actions name. On an edge with `-modifies(/x.num)`, a literal that reads
only `/x.num` holds after the commit if it held before. The facts at a
node are what every way into it carries, and nothing at an initial node.
An edge whose literals are `dead` together with its node's facts is taken
by no run. Checking a model does not refuse such an edge, because
contracts may end; the tools warn (`modality/dead-end-after-step`). Under
`V2` a rule check runs the flow from the rule's anchor node, seeded with
what accepted state at the anchor says about every path an edge
mentions, not from the model's initial states: after a model is
replaced, facts from those need not hold. The seed is true on the first
step only, so the check starts at a copy of the anchor with its edges
out and none in; a return to the anchor knows only what the edges into
it carry. It drops the edges the flow
rules out, and an edge meets a rule's labels only if the edge, the
labels, and the facts at its node can hold together. Lean proves the
seed holds (`stateFacts_seed`) and that no run from the anchor in that
state takes a dropped edge, or an edge with labels contradicting its
node's facts (`dead_after_sound`, whose literal list may be the edge's
and the labels'); case G19 is `g19_flip_never_taken`. That this leaves
every diamond and box a run can meet unchanged is argued, not proved:
formulas are not in the Lean spec.

## Checked claims

- `dead_sound`: if `dead` says the labels cannot hold together, no world
  satisfies them, whatever the opaque predicates mean
- `live_sound`: if `live` says they can, the world it built satisfies
  them; `live_not_dead`: the two verdicts never both fire
- `entails_sound`, and `entails_no`: a `no` comes with a world where the
  premises hold and the goal fails
- `deadIn_sound`, `liveIn_sound`: the same with accepted state known
- `consistent_dead`, `consistent_live`, `runtime_dead`, `runtime_live`:
  what each verdict on labels as written means, through elaboration
- `refund_edge_is_dead`, `release_edge_is_live`,
  `stricter_release_meets_the_rule`, `not_a_number_is_live`: the escrow
- `deadNaive_is_unsound`: a checker that always flips negated order
  literals would refuse an edge a commit can take
- `takesB_iff`: `takesB` is true exactly when some names take the edge;
  the faucet examples in `Vars.lean` (own slot yes, a neighbour's slot
  no) are decided by the kernel
- `carry_sound`: a literal over paths the edge frames survives the
  commit; `flow_sound`: closed facts hold at every node a run reaches,
  whatever the next commit is; `dead_after_sound`: labels `dead` with a
  node's facts are taken by no commit on any run; `closed_of_closedB`:
  `closedB` decides closure; case G6 (`g6_second_step_never_taken`) by the
  kernel
- `Generated.lean`: all 62 cases in `cases.json`, as 71 theorems and 15
  checked examples (each `unknown` is checked to stay `unknown`)

`#print axioms` reports `propext`, `Quot.sound`, and, for the `dead` and
`entails yes` proofs, `Classical.choice`. There is no `sorry` and no
`native_decide`; every case is decided by the kernel.

## Rust cross-check

```bash
cd rust
cargo test -p modality-lang --lib theory
cargo test -p modality-common --features model-governance witnesses_are_commits
```

The first runs every case in `cases.json` through the Rust theory. The
second draws random label sets and accepted states, takes every witness
the Rust theory returns, writes it as a state and a commit, and checks
that the governance evaluator (`predicate_holds`) accepts every label.

## Rust vs Lean on random label sets

```bash
cd experiments/predicate-theory/lean
./agree.sh                              # 20,000 sets
PT_ROUNDS=200000 PT_SEED=7 ./agree.sh
```

`rust_and_lean_agree` (ignored by default) draws label sets over every
sort: order between paths and decimal constants, `amount_in_range`,
booleans, text equality and substring tests, presence, `signed_by`,
`any_signed`, `all_signed`, `threshold` over nested prefixes and the
root, `modifies`, `post_to_path`, static labels, and an opaque custom
predicate, each possibly negated, and a known state for one set in
three. For each set, Lean elaborates the labels itself and must reach
the same literals and exactness, and the verdicts must match. Every
Rust witness must also pass Lean's check, and with a known state the
runtime verdicts must match. It also fails if a set with no opaque
label and an exact elaboration is left `unknown`. Eight seeds of 200,000
sets gave no disagreement and no such set.

`rust_and_lean_agree_on_variable_edges` draws edges over claimant slots
with variables and holes in `signed_by`, `modifies`, `post_to_path`,
`state_exists`, `bool_true`, `text_eq`, and `any_signed`, and a random
state and commit. The Rust search (`rust/modality-lang/src/vars.rs`)
tries fewer names than `takesB`: only segments at the position where the
variable sits, in the paths that predicate reads. Its answer must match
`takesB`. It runs 5,000 edges by default (`PT_ROUNDS` sets every
harness); three seeds of 20,000 gave no disagreement.

`rust_and_lean_agree_on_flow` draws models over four nodes with order,
boolean, text, presence, and signer labels and frequent `-modifies`
frames. Every other model starts from a random accepted state, whose
`state_facts` seed the flow. It sends the seed and the facts the Rust
flow (`theory/flow.rs`) computes to Lean. Lean must find them closed
(an initial node knowing at most the seed) and every edge Rust reports
`dead` with its node's facts; with `flow_sound`, no run takes a
reported edge. Four
seeds of 50,000 models (about 740 reported edges each) were all
certified.

## What this does not claim

- That model checking with variables is exact. The Rust checks a rule
  with variables against the model instantiated over the names the model
  and rule mention, plus one fresh name per variable. That rests on
  names being interchangeable, which is tested and not proved in Lean.
- That the Rust is verified. The harness samples; it does not prove the
  Rust follows the Lean. `pt-check` is compiled, so its answers trust
  Lean's compiler as well as the kernel.
- Completeness in general. `unknown` remains for opaque predicates,
  inexact declarations, and some signer sets: for example, keys that
  must differ (`-text_eq(/a.id,/b.id)`) under a `-threshold` bound, which
  is graph colouring and is not decided.
- That state flow finds every edge no run takes. A fact crosses an edge
  only when a `-modifies` label on that edge frames every path it reads.
  What a posted value says, and signer literals, are never carried, and
  an edge with variables carries nothing. The facts are sound, not the
  strongest.
- Enforcement. The network validates with `V0`; `V1` and `V2` refusals
  come from the `*_with_theory` entry points in `rust/modality-common`.
  Neither has been enforced on any network. `V1`'s dead verdicts were
  revised (text classes joined through literals, keys known by literal,
  static labels) before any network enforced it.
