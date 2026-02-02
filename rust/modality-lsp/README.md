# Modality LSP

Language Server Protocol implementation for `.modality` files.

## Features

- **Diagnostics**: Real-time parse error reporting
- **Completions**: Context-aware snippets for models, rules, formulas, predicates, and states
- **Hover**: Documentation for keywords, operators, predicates, and states
- **Go to Definition**: Jump to state and action definitions
- **Find References**: Find all usages of states and actions
- **Document Symbols**: Outline view showing models, rules, states, actions, and tests

## Installation

```bash
cargo install --path .
```

Or build from the workspace:

```bash
cd rust
cargo build --release -p modality-lsp
```

The binary will be at `target/release/modality-lsp`.

## Usage with VSCode

1. Install the Modality VSCode extension from `common/modality-vscode/`
2. Configure the extension to use the LSP:

```json
{
  "modality.lsp.path": "/path/to/modality-lsp"
}
```

## Usage with Other Editors

The LSP communicates over stdio. Configure your editor's LSP client to run:

```bash
modality-lsp
```

### Neovim (with nvim-lspconfig)

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

configs.modality = {
  default_config = {
    cmd = { 'modality-lsp' },
    filetypes = { 'modality' },
    root_dir = lspconfig.util.find_git_ancestor,
  },
}

lspconfig.modality.setup{}
```

### Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "modality"
scope = "source.modality"
file-types = ["modality"]
language-server = { command = "modality-lsp" }
```

### Zed

Add to settings:

```json
{
  "lsp": {
    "modality": {
      "binary": {
        "path": "modality-lsp"
      }
    }
  }
}
```

## Supported Capabilities

| Capability | Status |
|------------|--------|
| textDocument/didOpen | ✅ |
| textDocument/didChange | ✅ |
| textDocument/didClose | ✅ |
| textDocument/completion | ✅ |
| textDocument/hover | ✅ |
| textDocument/definition | ✅ |
| textDocument/references | ✅ |
| textDocument/documentSymbol | ✅ |
| textDocument/semanticTokens/full | ✅ |
| textDocument/formatting | ✅ |
| textDocument/publishDiagnostics | ✅ |

## Semantic Token Types

| Token | Applies To |
|-------|-----------|
| keyword | model, rule, formula, states, etc. |
| type | state names |
| function | predicates (signed_by, threshold) |
| operator | modal operators ([<+>], <+>) |
| variable | action names (UPPERCASE) |
| string | paths (/path/to/id) |
| number | numeric literals |
| comment | // comments |
| macro | temporal operators (always, eventually) |
| parameter | fixed-point variables (X in lfp) |

## Roadmap

- [ ] Code actions (quick fixes)
- [ ] Rename symbol
- [ ] Workspace symbols (cross-file navigation)
- [ ] Signature help
