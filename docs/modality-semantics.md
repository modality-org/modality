# Modality Language Semantics

This document explains the semantics of the Modality language, including how to interpret models, formulas, and the model checking process.

## Overview

Modality is a temporal logic language for describing and analyzing labeled transition systems (LTS). It allows you to:
- Define models as collections of graphs with labeled transitions
- Write temporal formulas to express properties about system behavior
- Check whether models satisfy these properties using model checking

## Core Concepts

### Models and Parts

A **model** represents a system as a collection of **parts**. Each part contains:
- **Nodes** (states of the system)
- **Transitions** between nodes with **labels** (actions)

```modality
model ExampleModel:
  part g1:
    n1 --> n2: +blue
    n2 --> n3: +green
    n3 --> n1: +red
```

### Transition Labels vs Formula Properties

**Important distinction:** Transition labels and formula properties have different semantics!

#### Transition Labels
Transition labels specify what the transition **requires or forbids**:
- `+blue`: The transition **requires** the `blue` action to be present
- `-red`: The transition **forbids** the `red` action (requires its absence)
- If a property isn't mentioned: The transition is **neutral** to that property

#### Formula Properties
Formula properties specify what actions the formula is **looking for**:
- `<+blue> phi`: "There exists a transition with the `blue` action that leads to a state satisfying `phi`"
- `[-red] phi`: "All transitions without the `red` action lead to states satisfying `phi`"

### The Key Insight: Neutral Transitions

If a transition doesn't mention a property at all, it's **usable** for that property:

```modality
model Example:
  graph g1:
    n1 --> n2: +blue    # Requires blue, neutral to yellow
    n2 --> n3: +green    # Requires green, neutral to yellow
    n3 --> n1: +red      # Requires red, neutral to yellow
```

For the formula `<+yellow> true`:
- All transitions are usable for `+yellow` because none explicitly forbid it
- The formula is **satisfied** because any state can make a transition with the `yellow` action

## Formula Syntax

### Boolean Operators
- `true`: Always satisfied
- `false`: Never satisfied
- `and`: Both subformulas must be satisfied
- `or`: At least one subformula must be satisfied
- `not`: Negation of the subformula
- `()`: Parentheses for grouping

### Modal Operators
- `<properties> phi`: **Diamond** operator - "There exists a transition with the specified properties that leads to a state satisfying `phi`"
- `[properties] phi`: **Box** operator - "All transitions with the specified properties lead to states satisfying `phi`"

### Property Lists
Multiple properties can be specified in modal operators:
- `<+blue +green> phi`: "There exists a transition that requires both blue AND green actions"
- `<+blue -red> phi`: "There exists a transition that requires blue AND forbids red"

## Model Checking Semantics

### Satisfaction Criteria

The model checker provides two satisfaction criteria:

1. **Per-Graph Requirement** (default): Formula is satisfied if at least one state from **each graph** satisfies it
2. **Any-State Requirement**: Formula is satisfied if at least one state **anywhere** satisfies it

### Property Satisfaction Rules

A transition satisfies a property list if it satisfies **all** properties in the list:

#### For `+property` (requires presence):
- ✅ Transition explicitly has `+property`
- ✅ Transition doesn't mention the property at all (neutral)

#### For `-property` (requires absence):
- ✅ Transition explicitly has `-property`
- ✅ Transition doesn't mention the property at all (neutral)

#### Examples:

```modality
model Test:
  graph g1:
    n1 --> n2: +blue +yellow    # Satisfies <+blue> and <+yellow>
    n2 --> n3: +blue -yellow     # Satisfies <+blue> and <-yellow>
    n3 --> n1: +red              # Satisfies <+blue> (neutral) and <+yellow> (neutral)
```

## Using the CLI

### Mermaid Diagram Generation

Generate visual diagrams of your models:

```bash
# Generate diagram for the first model in a file
modality model mermaid my-model.modality

# Generate diagram for a specific model
modality model mermaid my-model.modality --model ModelName
```

### Formula Checking

Check whether formulas are satisfied by models:

```bash
# Check a named formula
modality model check my-model.modality --model ModelName --formula FormulaName

# Check a formula text directly
modality model check my-model.modality --model ModelName --formula-text "<+blue> true"

# Check with default model (first model in file)
modality model check my-model.modality --formula FormulaName
```

## Examples

### Example 1: Basic Properties

```modality
model SimpleModel:
  graph g1:
    n1 --> n2: +blue
    n2 --> n3: +green
    n3 --> n1: +red

formula HasBlueTransition: <+blue> true
formula HasYellowTransition: <+yellow> true
```

**Results:**
- `HasBlueTransition`: ✅ Satisfied (n1 has explicit +blue transition)
- `HasYellowTransition`: ✅ Satisfied (all transitions are neutral to yellow)

### Example 2: Multiple Properties

```modality
model MultiPropertyModel:
  graph g1:
    n1 --> n2: +blue +yellow
    n2 --> n3: +blue -yellow
    n3 --> n1: +red

formula BlueAndYellow: <+blue +yellow> true
formula BlueNotYellow: <+blue -yellow> true
```

**Results:**
- `BlueAndYellow`: ✅ Satisfied (n1 transition has both +blue and +yellow)
- `BlueNotYellow`: ✅ Satisfied (n2 transition has +blue and -yellow)

### Example 3: Complex Formulas

```modality
model ComplexModel:
  graph g1:
    n1 --> n2: +blue
    n2 --> n3: +green
    n3 --> n1: +red

formula Complex: <+blue> <+green> true
```

**Result:**
- `Complex`: ✅ Satisfied (from n1, can take +blue to n2, then +green to n3)

## Common Patterns

### Safety Properties
Use box operators to express "always" properties:
```modality
formula AlwaysSafe: [all] safe
```

### Liveness Properties
Use diamond operators to express "eventually" properties:
```modality
formula EventuallyGood: <all> good
```

### Multi-Step Properties
Chain modal operators for complex behaviors:
```modality
formula EventuallyAlwaysGood: <all> [all] good
```

## Troubleshooting

### Formula Not Satisfied
- Check if transitions explicitly forbid the required properties
- Verify the model structure matches your expectations
- Use the CLI to see which states satisfy the formula

### Unexpected Results
- Remember the distinction between transition labels and formula properties
- Neutral transitions are usable for any property they don't explicitly mention
- The per-graph requirement means each graph must have at least one satisfying state

### Debugging Tips
- Start with simple formulas like `<+property> true`
- Use the CLI's detailed output to see which states satisfy formulas
- Build complex formulas incrementally
- Test with both per-graph and any-state criteria

## Further Reading

- [Modality Language Examples](../examples/language/)
- [CLI Documentation](../rust/modality/src/cmds/)
- [Model Checker Implementation](../rust/modality-lang/src/model_checker.rs) 