#!/usr/bin/env bash
# Build the pool program. Prints the wasm path and its sha256.
#
#   ./build.sh              build here, with the pinned toolchain
#   ./build.sh --canonical  build the published bytes: in a pinned Linux
#                           amd64 image, from a fixed path, so anyone who
#                           checks out this directory gets the same sha256
#
# A local build runs the same code, but cargo hashes the host and the
# package's path into symbol names, so its bytes differ from host to host.
set -euo pipefail
cd "$(dirname "$0")"

IMAGE="rust:1.94.1-slim@sha256:c6a474d7164ea2455e09b60a759b1edca38db7373c5689c1dae31780de4e71ac"

if [ "${1:-}" = "--canonical" ]; then
    mkdir -p target/canonical
    docker run --rm --platform linux/amd64 -v "$(pwd)":/in:ro -v "$(pwd)/target/canonical":/out "$IMAGE" \
        bash -c 'mkdir /src && cd /in && cp -r Cargo.toml Cargo.lock rust-toolchain.toml build.sh src /src/ && cd /src && ./build.sh >/dev/null && cp target/wasm32-unknown-unknown/release/constant_product_pool.wasm /out/'
    WASM="$(pwd)/target/canonical/constant_product_pool.wasm"
else
    # With rust-src installed, std's paths point into the local toolchain;
    # without it, at rustc's own /rustc/<commit>. Map the first to the second.
    SYSROOT=$(rustc --print sysroot)
    COMMIT=$(rustc -vV | sed -n 's/^commit-hash: //p')
    export RUSTFLAGS="--remap-path-prefix=$SYSROOT/lib/rustlib/src/rust=/rustc/$COMMIT --remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo --remap-path-prefix=$(pwd)=/src"
    cargo build -q --locked --release --target wasm32-unknown-unknown
    WASM="$(pwd)/target/wasm32-unknown-unknown/release/constant_product_pool.wasm"
fi

if command -v sha256sum >/dev/null; then
    SHA=$(sha256sum "$WASM" | cut -d' ' -f1)
else
    SHA=$(shasum -a 256 "$WASM" | cut -d' ' -f1)
fi
echo "$WASM $SHA"
