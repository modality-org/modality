---
sidebar_position: 1
title: Overview
---

# Language Reference

This document covers the complete syntax for Modality's model and rule definitions.

## File Types

| Extension | Purpose | Location |
|-----------|---------|----------|
| `.modality` | Model or rule definitions | `model/` or `rules/` |
| `.id` | Public identity (ed25519 pubkey) | `state/` |
| `.passfile` | Private key (for signing) | Project root |
| `.hash` | SHA256 hash commitment | `state/` |
| `.datetime` | ISO 8601 timestamp | `state/` |

## Quick Links

- [Model Syntax](./model-syntax) — Define state machines
- [Rule Syntax](./rule-syntax) — Define temporal constraints
- [Predicates](./predicates) — Cryptographic conditions
- [Path Types](./path-types) — Data type references
