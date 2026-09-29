# constant-product-pool

A two-asset constant-product pool as a Modality program, written against the
raw host ABI (`alloc`, `execute`, and `memory`; no imports). Operations:
`add`, `swap`, `remove`, and `refund`. The rules it runs under, the math,
and a walkthrough are in `docs/tutorials/constant-product-pool.md`;
`tests/network/amm-pool` runs it on a sequencer.

```bash
cargo test                 # the program's own tests, on the host
./build.sh                 # wasm32 build here; prints path and sha256
./build.sh --canonical     # the published bytes, built in a pinned Linux image
```

A local build runs the same code as the canonical one, but its bytes depend
on the host that built it: macOS and Linux, arm64 and amd64, each give
another sha256. A pool's rules name one sha256; publish the
canonical build so anyone can rebuild it and compare.
