# Change Log

## [Unreleased]

- Grammar rewritten against `grammar.lalrpop` and `rule_file.rs`: named
  rules, named formulas inside rules, legacy `a --> b: +label` labels,
  `[<+A>]`, `rule_for_this_commit`, inline commit models, `add_rule`, paths,
  strings and numbers in predicate arguments.
- Half-typed or old-syntax declarations no longer swallow the rest of the
  file.
- Semantic highlighting off by default for Modality files; the server's
  tokens predate the current syntax.
- The extension runs without `modality-lsp` instead of showing an error.
- The `.vsix` now carries the language client's dependencies.
- `install.sh` installs a `.vsix` through the editor CLIs; `npm run
  test:grammar` checks the grammar.
