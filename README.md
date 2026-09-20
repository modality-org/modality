# Modality ⲙ

<div align="center">
  <a href="https://modality.org">
    <img src="https://raw.githubusercontent.com/modality-org/modality/main/sites/www.modality.org/static/img/modality-social-card.png" alt="Modality Banner" width="600" />
  </a>
</div>

<div align="center">

⚙️ [Docs](https://www.modality.org/docs) | 🌟 [Examples](https://github.com/modality-org/modality/tree/main/examples) | 💬 [Discord](https://discord.gg/KpYFdrfnkS) | 💰 [Modal Money](https://www.modal.money)

</div>

> **Work in progress:** Modality is under active development. The language,
> verifier, CLI, docs, and examples are still changing, and some workflows may
> require building from source or using experimental commands.

## What is Modality?

Modality is a verification language for AI agent cooperation.

It enables agents (and humans*) to negotiate and verify cooperation through formal verification. Define modal contracts as append-only logs of signed commits, and prove commitments with temporal logic.

<sub>*Humans are also welcome to use Modality, if they're sufficiently motivated.</sub>

## Quick Start

```bash
curl --proto '=https' --tlsv1.2 -sSf https://www.modality.org/install.sh | sh
```

## Use Cases

- 🔐 **Modal Contracts** — State machines with formally verified temporal logic
- 🤖 **Agent Cooperation** — Escrow, swaps, milestones — provably enforced
- 📜 **Append-Only Logs** — Full history, transparent state, cryptographic integrity
- 🌐 **Decentralized Deployment** — Deploy contracts onto a global network via [Modal Money](https://www.modal.money)

## Documentation

- **[Getting Started](https://www.modality.org/docs/getting-started)** — Install and run your first contract
- **[Core Concepts](https://www.modality.org/docs/concepts)** — Understand models, formulas, and verification
- **[Language Reference](https://www.modality.org/docs/language)** — Complete syntax guide
- **[For Agents](https://www.modality.org/docs/for-agents)** — Quick reference for AI agents

## Development

Rust is the canonical implementation: language, verifier, CLI (`modal`), node, hub, and network (`modality-*` crates under [`/rust`](/rust)).

JavaScript is provided **as needed** for hosts that are not Rust — WASM, the TypeScript SDK, browsers, and Node agents. Prefer wrapping the Rust libraries (especially `modality-lang` WASM) over reimplementing them.

| | Path | Purpose |
|---|---|---|
| **Rust** | [`/rust`](/rust) | Canonical libraries and `modal` CLI |
| **JavaScript** | [`/js`](/js) | As-needed JS packages (`modality-js`), WASM, network/browser clients |
| **TypeScript SDK** | [`/packages/modality-sdk`](/packages/modality-sdk) | `@modality-org/sdk` |
| **VS Code** | [`/common/modality-vscode`](/common/modality-vscode) | LSP client for `modality-lsp` |

See [DEVELOPMENT.md](DEVELOPMENT.md) for local setup, build, and test instructions.

## Roadmap

- [Milestones](https://github.com/modality-org/modality/milestones)
- [Issues](https://github.com/modality-org/modality/issues)

## Community

- 💬 [Discord](https://discord.gg/KpYFdrfnkS) — Chat and community meetings
- 📂 [GitHub Issues](https://github.com/modality-org/modality/issues) — Report bugs or request features

---

<div align="center">

**Contributors**

<a href="https://github.com/modality-org/modality/graphs/contributors"><img src="https://contrib.rocks/image?repo=modality-org/modality" /></a>

[![Star History Chart](https://api.star-history.com/svg?repos=modality-org/modality&type=Date)](https://star-history.com/#modality-org/modality&Date)

</div>
