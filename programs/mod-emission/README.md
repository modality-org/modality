# mod-emission

The MOD contract's emission schedule as a Modality program, written against
the raw host ABI (`alloc`, `execute`, and `memory`; no imports). One
operation, `mint`, pays the subsidy of each named miner block, in index
order, from the parameters the genesis posted. The contract, its rules, and
the schedule are in `docs/concepts/mod-contract.md`;
`scripts/mod-genesis/build.sh` posts this program in a genesis.

```bash
cargo test                 # the program's own tests, on the host
./build.sh                 # wasm32 build here; prints path and sha256
./build.sh --canonical     # the published bytes, built in a pinned Linux image
```

A local build runs the same code as the canonical one, but its bytes depend
on the host that built it. A genesis names one sha256; a public network's
genesis should post the canonical build so anyone can rebuild it and
compare.
