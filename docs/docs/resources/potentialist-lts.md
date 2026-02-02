---
sidebar_position: 2
title: Potentialist LTS Paper
---

# Potentialist State Machines and Labeled Transition Systems

**Gerold Steiner**  
*Modality Project*  
February 2026

---

## Abstract

We introduce *potentialist labeled transition systems* (P-LTS) — a framework where a labeled transition system represents not a fixed structure but the *current actualization* of a potentially larger system. The system may grow through the addition of states and transitions, constrained only by an append-only sequence of *covenants*: hybrid temporal modal formulas that every actualization must satisfy. This approach bridges modal metaphysics (potentialism vs. actualism) with formal verification, providing semantics for evolving multi-party contracts where the rules accumulate but the structure remains open to cooperative extension.

---

## 1. Introduction

Traditional labeled transition systems (LTS) are static objects: a fixed set of states, a fixed set of labels, and a fixed transition relation. This works well for modeling closed systems — a vending machine, a protocol, a circuit. But many real-world coordination problems involve *open* systems that grow over time as participants negotiate new behaviors.

Consider a contract between two agents. Initially, they agree on basic rules: "Alice can deposit, Bob can withdraw after delivery." Later, they might add an arbiter for disputes. Later still, they might extend the contract to handle partial deliveries. The structure *evolves*, but earlier commitments must remain honored.

We propose **potentialist LTS** to capture this dynamic. The key insight: at any moment, the current LTS is one *actualization* of a space of *potential* LTSs. Covenants — expressed as hybrid temporal modal formulas — constrain which actualizations are valid. As covenants accumulate (append-only), the space of valid actualizations shrinks, but the system can still grow in any direction the covenants permit.

This paper develops the formal framework and connects it to practical contract verification.

---

## 2. Background: Labeled Transition Systems

### 2.1 Definition

A **labeled transition system** is a tuple `\mathcal\{L\} = (S, S_0, \Lambda, \rightarrow)` where:

- `S` is a set of states
- `S_0 \subseteq S` is the set of initial states
- `\Lambda` is a set of labels (actions)
- `\rightarrow \subseteq S \times \Lambda \times S` is the transition relation

We write `s \xrightarrow\{a\} s'` for `(s, a, s') \in \rightarrow`.

### 2.2 Modal Logic over LTS

The modal mu-calculus interprets formulas over LTS:

```
\phi ::= p \mid \neg\phi \mid \phi \land \phi \mid \langle a \rangle \phi \mid [a] \phi \mid \mu X. \phi \mid \nu X. \phi \mid X
```

- `\langle a \rangle \phi` — there exists an `a`-transition to a state satisfying `\phi`
- `[a] \phi` — all `a`-transitions lead to states satisfying `\phi`
- `\mu X. \phi` — least fixed point (reachability)
- `\nu X. \phi` — greatest fixed point (invariants)

A state `s` satisfies `\phi` (written `s \models \phi`) is defined inductively. An LTS `\mathcal\{L\}` satisfies `\phi` if all initial states satisfy it: `\mathcal\{L\} \models \phi \iff \forall s_0 \in S_0. s_0 \models \phi`.

---

## 3. Potentialism in Modal Metaphysics

### 3.1 Actualism vs. Potentialism

In the philosophy of mathematics, particularly set theory, a longstanding debate contrasts:

- **Actualism**: Mathematical objects exist as completed, definite totalities
- **Potentialism**: Mathematical objects are indefinitely extensible; there is no completed totality

The potentialist view, associated with Aristotle's distinction between actual and potential infinity, holds that the natural numbers, for instance, are not a completed set but a structure that can always be extended by "one more."

### 3.2 Modal Potentialism

Recent work (Linnebo, Studd, Hamkins) formalizes potentialism using modal logic:

- `\Diamond \phi` — it is *potentially* the case that `\phi`
- `\Box \phi` — it is *necessarily* the case that `\phi` (in all extensions)

A key principle is **directedness**: any two extensions have a common extension. This ensures coherence — the structure can grow in multiple directions without contradiction.

### 3.3 Application to LTS

We apply this perspective to LTS: the current system is *actual*, but it exists within a space of *potential* extensions. Adding states and transitions actualizes potential structure.

---

## 4. Potentialist LTS: Definition

### 4.1 LTS Extension

Given two LTS `\mathcal\{L\} = (S, S_0, \Lambda, \rightarrow)` and `\mathcal\{L\}' = (S', S'_0, \Lambda', \rightarrow')`, we say `\mathcal\{L\}'` **extends** `\mathcal\{L\}` (written `\mathcal\{L\} \preceq \mathcal\{L\}'`) iff:

1. `S \subseteq S'`
2. `S_0 = S'_0` (initial states preserved)
3. `\Lambda \subseteq \Lambda'`
4. `\rightarrow \subseteq \rightarrow'`

Extension is reflexive, transitive, and antisymmetric — a partial order on LTS.

### 4.2 Covenants

A **covenant** is a closed formula `\phi` in hybrid temporal modal logic. The **covenant language** extends mu-calculus with:

- **Nominals**: `@_s \phi` — `\phi` holds at named state `s`
- **Temporal anchoring**: `\mathsf\{starting\_at\}(s, \phi)` — `\phi` holds from state `s` onward
- **Hybrid binders**: `\downarrow x. \phi` — bind current state to `x`

### 4.3 Potentialist LTS

A **potentialist LTS** is a tuple `\mathcal\{P\} = (\mathcal\{L\}, \Gamma)` where:

- `\mathcal\{L\}` is the current (actual) LTS
- `\Gamma = [\phi_1, \phi_2, \ldots, \phi_n]` is an ordered, append-only list of covenants

The **validity condition**: `\mathcal\{L\} \models \bigwedge \Gamma` — the actual LTS satisfies all covenants.

### 4.4 Valid Extensions

Given `\mathcal\{P\} = (\mathcal\{L\}, \Gamma)`, an LTS `\mathcal\{L\}'` is a **valid extension** iff:

1. `\mathcal\{L\} \preceq \mathcal\{L\}'`
2. `\mathcal\{L\}' \models \bigwedge \Gamma`

The **potential space** of `\mathcal\{P\}` is:
```
\mathsf\{Pot\}(\mathcal\{P\}) = \\{ \mathcal\{L\}' \mid \mathcal\{L\} \preceq \mathcal\{L\}' \land \mathcal\{L\}' \models \bigwedge \Gamma \\}
```

### 4.5 Covenant Addition

Adding a covenant `\phi_\{n+1\}` to `\mathcal\{P\} = (\mathcal\{L\}, [\phi_1, \ldots, \phi_n])`:

```
\mathcal\{P\}' = (\mathcal\{L\}, [\phi_1, \ldots, \phi_n, \phi_\{n+1\}])
```

This is valid iff `\mathcal\{L\} \models \phi_\{n+1\}`. The new potential space satisfies:

```
\mathsf\{Pot\}(\mathcal\{P\}') \subseteq \mathsf\{Pot\}(\mathcal\{P\})
```

Covenants can only *shrink* the potential space, never expand it.

---

## 5. Hybrid Temporal Modal Formulas

### 5.1 Syntax

We use a hybrid extension of the modal mu-calculus:

```
\begin\{aligned\}
\phi ::= \; & p \mid \neg\phi \mid \phi \land \phi \mid \phi \lor \phi \mid \phi \to \phi \\
\mid \; & \langle a \rangle \phi \mid [a] \phi \mid \langle\langle a \rangle\rangle \phi \mid \langle\rangle\phi \mid []\phi \\
\mid \; & \mu X. \phi \mid \nu X. \phi \mid X \\
\mid \; & @_s \phi \mid \downarrow x. \phi \mid \mathsf\{starting\_at\}(s, \phi)
\end\{aligned\}
```

Where:
- `\langle\langle a \rangle\rangle \phi` is the **diamondbox**: `\neg\langle \bar\{a\} \rangle \top \land \langle a \rangle \phi` (committed to `a`)
- `@_s \phi` evaluates `\phi` at state `s`
- `\downarrow x. \phi` binds current state to `x`
- `\mathsf\{starting\_at\}(s, \phi)` anchors `\phi` to state `s`

### 5.2 Temporal Operators as Fixed Points

Standard temporal operators derive from fixed points:

```
\begin\{aligned\}
\mathsf\{always\}(\phi) &\equiv \nu X. \phi \land []X \\
\mathsf\{eventually\}(\phi) &\equiv \mu X. \phi \lor \langle\rangle X \\
\phi \; \mathsf\{until\} \; \psi &\equiv \mu X. \psi \lor (\phi \land \langle\rangle X)
\end\{aligned\}
```

### 5.3 Anchored Formulas

In potentialist LTS, formulas often anchor to specific states. The `\mathsf\{starting\_at\}` operator is crucial:

```
\mathsf\{starting\_at\}(s, \mathsf\{always\}(\phi))
```

This says: "from state `s` onward, `\phi` always holds." The covenant doesn't constrain states before `s`, allowing historical structure to be preserved while governing future evolution.

---

## 6. Semantic Properties

### 6.1 Monotonicity

**Theorem (Covenant Monotonicity)**: For any potentialist LTS `\mathcal\{P\} = (\mathcal\{L\}, \Gamma)` and covenant `\phi`:

```
\mathsf\{Pot\}((\mathcal\{L\}, \Gamma \cdot \phi)) \subseteq \mathsf\{Pot\}((\mathcal\{L\}, \Gamma))
```

*Proof*: Any `\mathcal\{L\}'` satisfying `\bigwedge(\Gamma \cdot \phi)` must satisfy `\bigwedge\Gamma`. `\square`

Covenants only restrict; the potential space never grows.

### 6.2 Consistency

A covenant list `\Gamma` is **consistent at** `\mathcal\{L\}` iff `\mathsf\{Pot\}((\mathcal\{L\}, \Gamma)) \neq \emptyset`.

**Theorem (Consistency Preservation)**: If `(\mathcal\{L\}, \Gamma)` is consistent and `\mathcal\{L\}'` is a valid extension, then `(\mathcal\{L\}', \Gamma)` is consistent.

*Proof*: `\mathcal\{L\}' \in \mathsf\{Pot\}((\mathcal\{L\}, \Gamma))`, so `\mathcal\{L\}' \in \mathsf\{Pot\}((\mathcal\{L\}', \Gamma))`. `\square`

### 6.3 Directedness

The potential space may or may not be directed (any two extensions have a common extension). In general:

**Proposition**: `\mathsf\{Pot\}(\mathcal\{P\})` is directed iff for all `\mathcal\{L\}_1, \mathcal\{L\}_2 \in \mathsf\{Pot\}(\mathcal\{P\})`, there exists `\mathcal\{L\}_3 \in \mathsf\{Pot\}(\mathcal\{P\})` with `\mathcal\{L\}_1, \mathcal\{L\}_2 \preceq \mathcal\{L\}_3`.

This depends on the covenants. Certain formulas (e.g., disjunctive invariants) can create branching potential spaces.

---

## 7. Satisfiability and Model Checking

### 7.1 The Satisfiability Problem

Given `(\mathcal\{L\}, \Gamma)` and a new covenant `\phi`, determine:

```
\exists \mathcal\{L\}' \succeq \mathcal\{L\}. \; \mathcal\{L\}' \models \bigwedge\Gamma \land \phi
```

This is the **potentialist satisfiability problem**: can we extend the current LTS to satisfy all covenants including the new one?

### 7.2 Decidability

**Theorem**: Potentialist satisfiability is decidable when:
1. The covenant language is the modal mu-calculus
2. Extensions are bounded (finite state/label additions)

*Proof sketch*: Reduce to mu-calculus satisfiability over finite LTS, which is EXPTIME-complete. `\square`

For unbounded extensions, satisfiability becomes undecidable in general.

### 7.3 Practical Approach: Model Witnesses

In Modality, we require a **model witness**: an explicit LTS demonstrating satisfiability. Adding a covenant requires providing an extended model that:

1. Extends the current model
2. Satisfies all existing covenants
3. Satisfies the new covenant

This shifts from decision problem to verification: check that the proposed witness is valid.

---

## 8. Labeled Actions and Predicates

### 8.1 Structured Labels

In practical P-LTS, labels carry structure beyond atomic names:

```
a ::= +p(args) \mid -p(args)
```

Where `p` is a predicate and `+/-` indicates polarity (must hold / must not hold).

### 8.2 Predicate Labels

Common predicates:
- `\mathsf\{signed\_by\}(\mathit\{path\})` — cryptographic signature
- `\mathsf\{threshold\}(n, \mathit\{path\})` — `n`-of-`m` multisig
- `\mathsf\{before\}(\mathit\{time\})` — temporal deadline
- `\mathsf\{oracle\}(\mathit\{source\}, \mathit\{claim\})` — external attestation

A transition `s \xrightarrow\{+\mathsf\{signed\_by\}(\mathtt\{/users/alice\})\} s'` requires Alice's signature to execute.

### 8.3 Label Satisfaction

An action `\alpha` **satisfies** label `a = +p(args)` iff predicate `p` evaluates to true on `\alpha`'s context. It satisfies `-p(args)` iff `p` evaluates to false.

---

## 9. Connection to Contracts

### 9.1 Contracts as P-LTS

A modal contract is a potentialist LTS where:
- States represent contract configurations
- Labels are signed actions (domain operations + signatures)
- Covenants are protection rules added by parties

### 9.2 The Commit Log

The **commit log** records the history of actualizations:

```
\mathcal\{L\}_0 \xrightarrow\{\phi_1\} \mathcal\{L\}_1 \xrightarrow\{\phi_2\} \mathcal\{L\}_2 \xrightarrow\{\alpha_1\} \mathcal\{L\}_3 \cdots
```

Each step either:
- Adds a covenant (extends `\Gamma`, may extend `\mathcal\{L\}`)
- Executes an action (transitions within current `\mathcal\{L\}`)

### 9.3 Multi-Party Evolution

When multiple parties cooperate:
1. Each adds their protective covenants
2. Covenants accumulate, constraining the space
3. The final potential space represents mutually acceptable evolutions
4. Execution occurs within this constrained space

The append-only nature ensures earlier protections cannot be revoked.

---

## 10. Examples

### 10.1 Simple Handshake

Initial P-LTS `\mathcal\{P\}_0`:
- `\mathcal\{L\}_0`: single state `\mathtt\{init\}`
- `\Gamma_0 = []`

Alice adds covenant:
```
\phi_A = \mathsf\{starting\_at\}(\mathtt\{init\}, \mathsf\{eventually\}(\mathtt\{done\}))
```

With model witness extending to:
```
init --[+signed_by(A)]--> a_ready --[+signed_by(B)]--> done
```

Bob adds covenant:
```
\phi_B = \mathsf\{starting\_at\}(\mathtt\{init\}, \mathsf\{eventually\}(\mathtt\{done\}))
```

Same formula — the potential space now requires reaching `\mathtt\{done\}`.

Execution: Alice signs (→ `\mathtt\{a\_ready\}`), Bob signs (→ `\mathtt\{done\}`). Both covenants satisfied.

### 10.2 Escrow with Extension

Initial model:
```
init --[+signed_by(buyer)]--> deposited --[+signed_by(seller)]--> delivered --[+signed_by(buyer)]--> released
```

Covenant: "release requires prior delivery"
```
\mathsf\{always\}([\mathtt\{release\}] \Rightarrow \langle\mathtt\{deliver\}\rangle\top)
```

Later, parties agree to add dispute resolution. Extended model:
```
deposited --[+signed_by(buyer)]--> disputed --[+signed_by(arbiter)]--> resolved
disputed --[+signed_by(arbiter)]--> released
```

New covenant: "arbiter resolution respects delivery status"
```
@_\{\mathtt\{disputed\}\}([\mathtt\{release\}] \Rightarrow \langle\mathtt\{deliver\}\rangle\top)
```

The extension is valid — it satisfies all existing covenants while adding new structure.

### 10.3 Incompatible Extension Attempt

Suppose Alice's covenant requires:
```
\mathsf\{always\}([\mathtt\{+withdraw\}] \Rightarrow \langle\mathtt\{+signed\_by(alice)\}\rangle\top)
```

Bob attempts to add a model where:
```
funded --[+signed_by(bob)]--> withdrawn
```

This extension is **invalid** — it violates Alice's covenant. The potentialist framework rejects it.

---

## 11. Related Work

### 11.1 Process Algebra

CCS, CSP, and the π-calculus model concurrent processes with labeled transitions. Our work differs in:
- Focus on *evolution* of the LTS itself, not just state
- Covenants as first-class constraints on evolution
- Append-only accumulation of constraints

### 11.2 Modal Specifications

Larsen's modal transition systems distinguish *may* and *must* transitions. P-LTS extends this with:
- Arbitrary modal formulas (not just may/must)
- Ordered accumulation (historical structure)
- Anchored formulas with hybrid operators

### 11.3 Contract Logics

Deontic logics (obligations, permissions) model normative systems. P-LTS provides:
- Operational semantics (explicit state machines)
- Verification via model checking
- Cryptographic grounding (signatures in labels)

### 11.4 Potentialist Set Theory

Linnebo and Studd's modal set theory inspires our framework. We adapt:
- The potential/actual distinction
- Modal operators over extensions
- Directedness as a coherence condition

---

## 12. Conclusion

Potentialist labeled transition systems provide a formal foundation for evolving, multi-party coordination structures. By treating the current LTS as one actualization within a space of potentials — constrained by an append-only sequence of hybrid temporal modal covenants — we capture the dynamics of negotiated cooperation.

Key contributions:
1. **P-LTS definition**: LTS + ordered covenant list
2. **Validity via model witnesses**: Explicit models demonstrate satisfiability
3. **Monotonic constraint accumulation**: Covenants only shrink the potential space
4. **Connection to contracts**: P-LTS semantics for multi-party agreements

Future work includes:
- Efficient satisfiability algorithms for bounded extensions
- Compositional P-LTS for modular contracts
- Probabilistic extensions for uncertain cooperation
- Implementation in the Modality verification system

The potentialist perspective — that structures are never final, only constrained — aligns naturally with open-ended cooperation. Trust emerges not from fixing the future, but from accumulating guarantees about what any future must satisfy.

---

## References

1. Baier, C., & Katoen, J. P. (2008). *Principles of Model Checking*. MIT Press.

2. Blackburn, P., de Rijke, M., & Venema, Y. (2001). *Modal Logic*. Cambridge University Press.

3. Bradfield, J., & Stirling, C. (2007). Modal mu-calculi. In *Handbook of Modal Logic* (pp. 721-756). Elsevier.

4. Hamkins, J. D. (2018). The modal logic of arithmetic potentialism and the universal algorithm. *arXiv:1801.04599*.

5. Larsen, K. G. (1990). Modal specifications. In *Automatic Verification Methods for Finite State Systems* (pp. 232-246). Springer.

6. Linnebo, Ø. (2013). The potential hierarchy of sets. *The Review of Symbolic Logic*, 6(2), 205-228.

7. Milner, R. (1989). *Communication and Concurrency*. Prentice Hall.

8. Studd, J. P. (2019). *Everything, More or Less: A Defence of Generality Relativism*. Oxford University Press.

---

## Further Reading

- [Bud Mishra](https://scholar.google.com/citations?user=kXVBr20AAAAJ&hl=en&oi=ao) — Father of hardware formal verification

---

*The actual is carved from the potential by covenant.* 🔐
