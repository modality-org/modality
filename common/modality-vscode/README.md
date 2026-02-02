# Modality VSCode Extension

Language support for Modality - modal contracts and verification.

## Features

- **Syntax Highlighting**: TextMate grammar for `.modality` files
- **Diagnostics**: Real-time parse error detection
- **Completions**: Snippets for models, rules, formulas, predicates
- **Hover**: Documentation for keywords and operators
- **Go to Definition**: Navigate to state and action definitions
- **Find References**: Find all usages of symbols
- **Document Symbols**: Outline view (Ctrl+Shift+O)
- **Semantic Tokens**: Enhanced highlighting
- **Formatting**: Format document (Shift+Alt+F)
- **Code Actions**: Quick fixes and refactorings

## Requirements

The extension requires the `modality-lsp` language server. Install it:

```bash
# From the modality repo
cd rust
cargo install --path modality-lsp

# Or from crates.io (when published)
cargo install modality-lsp
```

## Installation

### From VSIX

1. Download the `.vsix` file from releases
2. In VSCode: Extensions → ... → Install from VSIX

### From Source

```bash
cd common/modality-vscode
npm install
npm run compile
npm run package
code --install-extension modality-vscode-*.vsix
```

### Manual Install (Development)

```bash
./install.sh
```

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `modality.lsp.enabled` | `true` | Enable the language server |
| `modality.lsp.path` | `modality-lsp` | Path to LSP binary |
| `modality.lsp.trace.server` | `off` | Trace LSP communication |

## Commands

| Command | Description |
|---------|-------------|
| `Modality: Restart Language Server` | Restart the LSP |

## Example

```modality
model escrow {
  states { idle, funded, complete }
  initial { idle }
  transitions {
    idle -[DEPOSIT]-> funded
    funded -[RELEASE]-> complete
  }
}

export default rule {
  starting_at $PARENT
  formula {
    always (
      [<+RELEASE>] signed_by(/parties/seller.id)
    )
  }
}
```

## Development

```bash
# Watch mode
npm run watch

# Package
npm run package
```

## Links

- [Modality Documentation](https://docs.modality.org)
- [Modality GitHub](https://github.com/modality-org/modality)
- [Language Server](../../rust/modality-lsp/)
