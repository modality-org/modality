# Modality LSP

Language Server Protocol implementation for `.modality` files.

## Features

- **Diagnostics**: Real-time parse error reporting
- **Completions**: Snippets for models, rules, formulas, and predicates
- **Hover**: Documentation for keywords, operators, and predicates

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

## Supported Capabilities

| Capability | Status |
|------------|--------|
| textDocument/didOpen | ✅ |
| textDocument/didChange | ✅ |
| textDocument/didClose | ✅ |
| textDocument/completion | ✅ |
| textDocument/hover | ✅ |
| textDocument/publishDiagnostics | ✅ |

## Roadmap

- [ ] Go to definition for states and actions
- [ ] Find references
- [ ] Document symbols
- [ ] Semantic tokens (enhanced highlighting)
- [ ] Code actions (quick fixes)
- [ ] Formatting
