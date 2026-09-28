# modality-validator

Contract replay and prefix attestation for Modality validators, the third
node role beside miners and sequencers.

A validator replays a contract prefix (signatures, model, rules, predicates)
through a named commit and signs a prefix certificate. Dest REPOST and RECV
apply consume a quorum of those certificates when the network sets
`repost_requires_validator_cert`. Validators do not mine or sequence; run one
with `modal node run-validator`.

Ordering (Narwhal/Shoal) lives in `modality-sequencer`. Sequencers and
observers use the same `ContractProcessor` from this crate to apply ordered
commits.

## Modules

| Module | Purpose |
|--------|---------|
| `contract_processor` | Applies commits to contract state (POST, RULE, assets, REPOST, RECV) |
| `modality_processor` | Model and rule checks for Modality contracts |
| `sequenced_rules` | Checks a pending commit against its sequenced parent chain's rules and the network's predicate theory |
| `predicate_executor` | Predicate evaluation, including replay evidence |
| `program_executor` / `invoke_engine` | WASM programs invoked from commits |
| `prefix_cert` | Prefix certificate type, signing, verification, and quorum counting |

## Network fields

Network config lists the validator set under `validators` (distinct from the
`sequencers` committee), with `validator_min_stake`, `validation_fees`,
`validator_qc_numerator`, and `validator_qc_denominator`.

## Testing

```bash
cargo test -p modality-validator
```
