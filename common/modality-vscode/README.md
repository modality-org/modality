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
- **Formatting**: Format document (Shift+Alt+F)
- **Code Actions**: Quick fixes and refactorings

## Requirements

Highlighting needs nothing else. The other features come from the
`modality-lsp` language server; without it on your `PATH` (or set in
`modality.lsp.path`) the extension runs without them. Install it:

```bash
# From the modality repo
cd rust
cargo install --path modality-lsp

# Or from crates.io (when published)
cargo install modality-lsp
```

## Installation

From the repo:

```bash
cd common/modality-vscode
./install.sh            # or: npm install && npm run install:local
```

This checks the grammar, builds `modality-vscode.vsix`, and installs it with
whichever of `code`, `cursor`, `codium` is on your `PATH`. Then run
**Developer: Reload Window**. `./uninstall.sh` removes it.

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `modality.lsp.enabled` | `true` | Enable the language server |
| `modality.lsp.path` | `modality-lsp` | Path to LSP binary |
| `modality.lsp.trace.server` | `off` | Trace LSP communication |

The extension sets `editor.semanticHighlighting.enabled` to `false` for
Modality files, so the grammar does all highlighting. Turn it back on under
`[modality]` in your settings to see the server's semantic tokens.

## Commands

| Command | Description |
|---------|-------------|
| `Modality: Restart Language Server` | Restart the LSP |

## Example

```modality
model Escrow {
  initial idle
  idle -> funded [+DEPOSIT +signed_by(/parties/buyer.id)]
  funded -> done [+RELEASE +signed_by(/parties/seller.id)]
}

export default rule {
  starting_at $PARENT
  formula {
    always([+RELEASE -signed_by(/parties/seller.id)] false)
  }
}
```

## Development

The grammar is `syntaxes/modality.tmLanguage.json`. It follows
`rust/modality-lang/src/grammar.lalrpop` (models, formulas, actions, tests,
contracts) and `rust/modality-lang/src/rule_file.rs` (rule files); change it
when they change.

```bash
npm run test:grammar    # scope checks + every .modality file in the repo
npm run watch           # rebuild src/ on change
```

`npm run test:grammar` tokenizes `test/fixtures/all-syntax.modality` with the
same engine VS Code uses and checks each construct's scope. It also checks
that half-typed declarations don't swallow the rest of the file, and that
every `.modality` file in the repo closes all its blocks by the end.
Add a line to the fixture and to `EXPECT` in `test/grammar.test.js` for
any syntax you add.

To try changes live, open this folder in VS Code and press F5 (Extension
Development Host), then run **Developer: Reload Window** in that window after
each grammar edit. **Developer: Inspect Editor Tokens and Scopes** shows the
scope under the cursor.

## Links

- [Modality Documentation](https://docs.modality.org)
- [Modality GitHub](https://github.com/modality-org/modality)
- [Language Server](../../rust/modality-lsp/)
