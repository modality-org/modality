---
sidebar_position: 1
title: Overview
---

# Language Reference

Syntax for models, rules, predicates, and typed paths. Read the cookbooks
first when you are writing a formula or a witness model.

A contract is **state + model + rules**. This reference is the language those
artifacts are written in.

## File Types

| Extension | Purpose | Location |
|-----------|---------|----------|
| `.modality` | Model or rule definitions | `model/` or `rules/` |
| `.id` | Public identity (ed25519 pubkey) | `state/` |
| `.passfile` | Private key (for signing) | Project root |
| `.hash` | SHA256 hash commitment | `state/` |
| `.datetime` | ISO 8601 timestamp | `state/` |

## Quick Links

- [Formula Cookbook](./formula-cookbook) — NL → one rule formula (read this first)
- [Model Cookbook](./model-cookbook) — Witness LTS recipes
- [Model Syntax](./model-syntax) — Define labeled transition systems
- [Rule Syntax](./rule-syntax) — Define temporal constraints
- [Predicates](./predicates) — Cryptographic conditions
- [Path Types](./path-types) — Data type references
