# LLM-Enhanced Model Synthesizer

An experimental tool that uses Claude to synthesize Modality contracts from natural language descriptions.

## Overview

This synthesizer takes natural language descriptions of agent cooperation scenarios and generates:
1. Modality models (state machines)
2. Temporal logic formulas (rules)
3. Protection guarantees for each party

## Usage

```bash
# Single-shot synthesis
./synthesize.sh "Alice wants to hire Bob for a task. She pays after he delivers."

# Interactive mode
./synthesize.sh --interactive

# With validation
./synthesize.sh --validate "Alice and Bob want to trade tokens atomically"
```

## Architecture

1. **Context Injection**: Feed Claude the Modality syntax, examples, and patterns
2. **NL Analysis**: Extract parties, actions, trust requirements, and flow
3. **Model Generation**: Synthesize a state machine that satisfies the requirements
4. **Rule Generation**: Create formulas that protect each party
5. **Validation**: Parse the output and check for errors
6. **Iteration**: If validation fails, refine with Claude

## Examples

See `examples/` for sample inputs and outputs.
