## Developer README

This guide covers local setup, building, testing, and common developer workflows for Modality across Rust and JavaScript workspaces.

### Prerequisites

- Node.js ≥ 18.1 (recommend using `fnm`, `nvm`, or `asdf`)
- pnpm 9.x (repo pins `pnpm@9.3.0`)
- Rust (stable toolchain; install via `rustup`)
- macOS or Linux (Windows via WSL)
- CMake (required for native crates like `randomx-rs`)
- Xcode Command Line Tools on macOS (`xcode-select --install`)
- GNU coreutils on macOS (for `timeout` command in network example tests)

Recommended:

- corepack (ships with Node 16.13+)

Enable pnpm via corepack to match the repo’s pinned version:

```bash
corepack enable
corepack prepare pnpm@9.3.0 --activate
```

### Repository Layout (high level)

- `rust/` Rust workspace: core language, CLI, node, validator, etc.
- `js/` JavaScript monorepo: network, node, datastore, viewer, CLI, etc.
- `examples/` runnable examples (language, network, mining)
- `fixtures/` sample configs and passfiles
- `docs/` reference and design docs (source of truth for the public docs site)
- `sites/www.modality.org/` Docusaurus website that publishes `docs/`
- `scripts/` release/build helpers

---

## Quick Start (all-in-one)

```bash
# Rust (build + test)
cd rust
cargo build --release
cargo test

# JavaScript (install deps + build + test)
cd ../js
pnpm i -r
pnpm run build
pnpm run test
```

---

## Rust Development

Workspace is defined in `rust/Cargo.toml`. Build and test everything:

```bash
cd rust
cargo build --release
cargo test
```

Useful commands:

```bash
# Build debug (faster iterative builds)
cargo build

# Run CLI directly
cargo run -p modality -- --help

# Install CLI locally from workspace
cargo install --path modality

# Lint/format (recommended before PRs)
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

Convenience scripts:

```bash
# Same as above but wrapped
rust/scripts/build   # cargo build --release
rust/scripts/test    # cargo test
```

Artifacts:

- CLI binary (after release build): `rust/target/release/modality`

---

## JavaScript Development

The JS workspace uses pnpm workspaces + lerna.

```bash
cd js
pnpm i -r          # install all workspaces
pnpm run build     # lerna run build across packages
pnpm run test      # lerna run test across packages
```

Convenience scripts:

```bash
js/scripts/build    # pnpm i -r
js/scripts/test     # pnpm i -r && pnpm run -r test --passWithNoTests
```

Notes:

- Engines: Node ≥ 18.1, pnpm ≥ 8.14.1 (repo pins pnpm 9.3.0).
- Use `pnpm run -r <script>` to execute a script across all packages.
- Individual packages can be built/tested with filters, e.g.:

```bash
pnpm --filter @modality-dev/network-node run build
pnpm --filter @modality-dev/network-node run test
```

---

## Documentation site

Edit markdown in repo-root `docs/`. That tree is what GitHub Pages publishes: CI copies it into `sites/www.modality.org/docs` before building. Do not treat `sites/www.modality.org/docs/` as the source of truth; deploy overwrites it from `docs/`.

Preview locally with:

```bash
./scripts/run-site.sh
```

The script requires Node.js ≥ 20. It copies `docs/` into the Docusaurus site (same as CI), installs npm dependencies if needed, and starts the dev server. Open http://localhost:3000 (first contract: http://localhost:3000/docs/getting-started/first-contract).

While reviewing and changing docs:

1. Edit files under `docs/` (for example `docs/getting-started/first-contract.md`).
2. Re-run `./scripts/run-site.sh` so the copy is fresh, then reload the browser. The copy happens at startup, so a running server will not see new `docs/` edits until you restart it.
3. Site chrome (sidebar, theme, blog, `docusaurus.config.ts`) lives in `sites/www.modality.org/`. Those files live-reload. If the server is already running and you are only changing chrome, restart with `./scripts/run-site.sh --no-sync` to skip another docs copy.
4. Commit content changes in `docs/`. Commit sidebar or theme changes in `sites/www.modality.org/` (not a one-off copy under `sites/www.modality.org/docs/`).

Useful flags (passed through to Docusaurus unless noted):

```bash
./scripts/run-site.sh --help
./scripts/run-site.sh --port 3001
./scripts/run-site.sh --no-sync          # skip copying docs/
```

---

## Examples

Language examples:

```bash
cd examples/language/01-simple-models
./01-generate-diagram.sh
./02-check-formulas.sh
```

Network examples (devnets):

```bash
cd examples/network/01-ping-node
./01-run-node1.sh
./02-ping-node1-from-node2.sh
```

More scenarios are available under `examples/network/*` and `examples/network/05-mining`.
Refer to `examples/network/VERIFICATION.md` for verification steps and `SCRIPTS_UPDATE_SUMMARY.md` for script notes.

---

## Fixtures

- Network configs: `fixtures/network-configs/*` and `fixtures/network-node-configs/*`
- Passfiles for local testing: `fixtures/passfiles/*`

Use these fixtures with the JS node commands or Rust node tools as needed.

---

## Formatting, Linting, Testing

Rust:

```bash
cd rust
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

JavaScript:

```bash
cd js
pnpm run lint
pnpm run format
pnpm run test
```

---

## Troubleshooting

- `cmake` not found (e.g. building `randomx-rs` on macOS):

  ```bash
  # Ensure Xcode Command Line Tools are installed
  xcode-select --install

  # Install cmake
  brew install cmake

  # Verify
  cmake --version

  # Rebuild
  cd rust && cargo build --release
  ```
- `timeout` command not found (running network example tests on macOS):

  ```bash
  # The timeout command is part of GNU coreutils, not included on macOS by default
  # Install via Homebrew
  brew install coreutils

  # Verify (GNU timeout is installed as gtimeout)
  gtimeout --version
  ```
- pnpm version mismatch:

  ```bash
  corepack enable
  corepack prepare pnpm@9.3.0 --activate
  ```
- Build errors after branch switch: clean and rebuild

  ```bash
  cd rust && cargo clean && cargo build
  cd ../js && rm -rf node_modules && pnpm i -r && pnpm run build
  ```
- macOS permissions for scripts:

  ```bash
  chmod +x rust/scripts/* js/scripts/* examples/**/**/*.sh
  ```

---

## Useful Links

- Root README: `README.md`
- Local docs preview: `./scripts/run-site.sh` (see [Documentation site](#documentation-site))
- Developer Guide (architecture/extension): `docs/developer-guide.md`
- Quick Reference: `docs/quick-reference.md`
- Language semantics: `docs/modality-semantics.md`
