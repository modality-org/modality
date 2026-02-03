use dashmap::DashMap;
use ropey::Rope;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use modality_lang::parse_content_lalrpop;

mod semantic_tokens;
mod formatter;
mod code_actions;

use semantic_tokens::{compute_semantic_tokens, tokens_to_lsp, get_legend};
use formatter::format_document;
use code_actions::compute_code_actions;

/// Document state stored for each open file
struct Document {
    content: Rope,
    #[allow(dead_code)]
    version: i32,
    /// Cached symbols extracted from the document
    symbols: Vec<DocumentSymbolInfo>,
}

/// Information about a symbol in the document
#[derive(Clone, Debug)]
struct DocumentSymbolInfo {
    name: String,
    kind: SymbolKind,
    range: Range,
    selection_range: Range,
    children: Vec<DocumentSymbolInfo>,
}

/// The Modality Language Server
struct ModalityLanguageServer {
    client: Client,
    documents: DashMap<String, Document>,
}

impl ModalityLanguageServer {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
        }
    }

    /// Parse document and return diagnostics
    async fn diagnose(&self, _uri: &Url, text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        
        // Try to parse using the lalrpop parser
        match parse_content_lalrpop(text) {
            Ok(_) => {
                // Parsed successfully, no diagnostics
            }
            Err(e) => {
                diagnostics.push(Diagnostic {
                    range: error_to_range(&e, text),
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("modality".to_string()),
                    message: format!("Parse error: {}", e),
                    ..Default::default()
                });
            }
        }
        
        diagnostics
    }

    /// Extract symbols from the document for outline and navigation
    fn extract_symbols(&self, text: &str) -> Vec<DocumentSymbolInfo> {
        let mut symbols = Vec::new();
        let lines: Vec<&str> = text.lines().collect();
        
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();
            
            // Model definition
            if trimmed.starts_with("model ") {
                if let Some(model_info) = self.parse_model_symbol(&lines, i) {
                    symbols.push(model_info.0);
                    i = model_info.1;
                    continue;
                }
            }
            
            // Rule definition
            if trimmed.starts_with("rule") || trimmed.starts_with("export default rule") {
                if let Some(rule_info) = self.parse_rule_symbol(&lines, i) {
                    symbols.push(rule_info.0);
                    i = rule_info.1;
                    continue;
                }
            }
            
            // Formula definition
            if trimmed.starts_with("formula") {
                if let Some(formula_info) = self.parse_formula_symbol(&lines, i) {
                    symbols.push(formula_info.0);
                    i = formula_info.1;
                    continue;
                }
            }
            
            // Action definition
            if trimmed.starts_with("action ") {
                if let Some(action_info) = self.parse_action_symbol(&lines, i) {
                    symbols.push(action_info.0);
                    i = action_info.1;
                    continue;
                }
            }
            
            // Test definition
            if trimmed.starts_with("test ") {
                if let Some(test_info) = self.parse_test_symbol(&lines, i) {
                    symbols.push(test_info.0);
                    i = test_info.1;
                    continue;
                }
            }
            
            i += 1;
        }
        
        symbols
    }

    /// Parse a model definition and return symbol info and end line
    fn parse_model_symbol(&self, lines: &[&str], start: usize) -> Option<(DocumentSymbolInfo, usize)> {
        let line = lines[start];
        let trimmed = line.trim();
        
        // Extract model name
        let name = trimmed
            .strip_prefix("model ")?
            .split('{')
            .next()?
            .trim()
            .to_string();
        
        // Find the end of the model (matching braces)
        let end = find_block_end(lines, start)?;
        
        // Extract children (states, transitions)
        let mut children = Vec::new();
        
        for i in start..=end {
            let child_line = lines[i].trim();
            
            // States block
            if child_line.starts_with("states") {
                if let Some(states) = self.extract_states(lines, i) {
                    children.extend(states);
                }
            }
            
            // Transitions - extract action names
            if child_line.contains("-[") && child_line.contains("]->") {
                if let Some(action) = extract_transition_action(child_line) {
                    let col_start = lines[i].find(&action).unwrap_or(0);
                    children.push(DocumentSymbolInfo {
                        name: action.clone(),
                        kind: SymbolKind::EVENT,
                        range: Range {
                            start: Position { line: i as u32, character: col_start as u32 },
                            end: Position { line: i as u32, character: (col_start + action.len()) as u32 },
                        },
                        selection_range: Range {
                            start: Position { line: i as u32, character: col_start as u32 },
                            end: Position { line: i as u32, character: (col_start + action.len()) as u32 },
                        },
                        children: vec![],
                    });
                }
            }
        }
        
        Some((DocumentSymbolInfo {
            name,
            kind: SymbolKind::CLASS,
            range: Range {
                start: Position { line: start as u32, character: 0 },
                end: Position { line: end as u32, character: lines[end].len() as u32 },
            },
            selection_range: Range {
                start: Position { line: start as u32, character: 0 },
                end: Position { line: start as u32, character: line.len() as u32 },
            },
            children,
        }, end + 1))
    }

    /// Extract state symbols from a states block
    fn extract_states(&self, lines: &[&str], start: usize) -> Option<Vec<DocumentSymbolInfo>> {
        let mut states = Vec::new();
        let line = lines[start];
        
        // Single line: states { s0, s1, s2 }
        if line.contains('{') && line.contains('}') {
            let content = line.split('{').nth(1)?.split('}').next()?;
            for state in content.split(',') {
                let state_name = state.trim();
                if !state_name.is_empty() {
                    let col = line.find(state_name).unwrap_or(0);
                    states.push(DocumentSymbolInfo {
                        name: state_name.to_string(),
                        kind: SymbolKind::ENUM_MEMBER,
                        range: Range {
                            start: Position { line: start as u32, character: col as u32 },
                            end: Position { line: start as u32, character: (col + state_name.len()) as u32 },
                        },
                        selection_range: Range {
                            start: Position { line: start as u32, character: col as u32 },
                            end: Position { line: start as u32, character: (col + state_name.len()) as u32 },
                        },
                        children: vec![],
                    });
                }
            }
        }
        
        Some(states)
    }

    /// Parse a rule definition
    fn parse_rule_symbol(&self, lines: &[&str], start: usize) -> Option<(DocumentSymbolInfo, usize)> {
        let line = lines[start];
        let end = find_block_end(lines, start)?;
        
        let name = if line.contains("export default") {
            "default rule".to_string()
        } else {
            "rule".to_string()
        };
        
        Some((DocumentSymbolInfo {
            name,
            kind: SymbolKind::FUNCTION,
            range: Range {
                start: Position { line: start as u32, character: 0 },
                end: Position { line: end as u32, character: lines[end].len() as u32 },
            },
            selection_range: Range {
                start: Position { line: start as u32, character: 0 },
                end: Position { line: start as u32, character: line.len() as u32 },
            },
            children: vec![],
        }, end + 1))
    }

    /// Parse a formula definition
    fn parse_formula_symbol(&self, lines: &[&str], start: usize) -> Option<(DocumentSymbolInfo, usize)> {
        let line = lines[start];
        let end = find_block_end(lines, start)?;
        
        Some((DocumentSymbolInfo {
            name: "formula".to_string(),
            kind: SymbolKind::FUNCTION,
            range: Range {
                start: Position { line: start as u32, character: 0 },
                end: Position { line: end as u32, character: lines[end].len() as u32 },
            },
            selection_range: Range {
                start: Position { line: start as u32, character: 0 },
                end: Position { line: start as u32, character: line.len() as u32 },
            },
            children: vec![],
        }, end + 1))
    }

    /// Parse an action definition
    fn parse_action_symbol(&self, lines: &[&str], start: usize) -> Option<(DocumentSymbolInfo, usize)> {
        let line = lines[start];
        let trimmed = line.trim();
        
        let name = trimmed
            .strip_prefix("action ")?
            .split('{')
            .next()?
            .trim()
            .to_string();
        
        let end = find_block_end(lines, start)?;
        
        Some((DocumentSymbolInfo {
            name,
            kind: SymbolKind::METHOD,
            range: Range {
                start: Position { line: start as u32, character: 0 },
                end: Position { line: end as u32, character: lines[end].len() as u32 },
            },
            selection_range: Range {
                start: Position { line: start as u32, character: 0 },
                end: Position { line: start as u32, character: line.len() as u32 },
            },
            children: vec![],
        }, end + 1))
    }

    /// Parse a test definition
    fn parse_test_symbol(&self, lines: &[&str], start: usize) -> Option<(DocumentSymbolInfo, usize)> {
        let line = lines[start];
        let trimmed = line.trim();
        
        let name = trimmed
            .strip_prefix("test ")?
            .split('{')
            .next()?
            .trim()
            .trim_matches('"')
            .to_string();
        
        let end = find_block_end(lines, start)?;
        
        Some((DocumentSymbolInfo {
            name,
            kind: SymbolKind::METHOD,
            range: Range {
                start: Position { line: start as u32, character: 0 },
                end: Position { line: end as u32, character: lines[end].len() as u32 },
            },
            selection_range: Range {
                start: Position { line: start as u32, character: 0 },
                end: Position { line: start as u32, character: line.len() as u32 },
            },
            children: vec![],
        }, end + 1))
    }

    /// Find definition of a symbol at position
    fn find_definition(&self, text: &str, position: Position) -> Option<Location> {
        let word = get_word_at_position(text, position)?;
        let lines: Vec<&str> = text.lines().collect();
        
        // Search for state definitions
        for (i, line) in lines.iter().enumerate() {
            // In states block
            if line.contains("states") && line.contains(&word) {
                let col = line.find(&word)?;
                return Some(Location {
                    uri: Url::parse("file:///").ok()?, // Will be replaced with actual URI
                    range: Range {
                        start: Position { line: i as u32, character: col as u32 },
                        end: Position { line: i as u32, character: (col + word.len()) as u32 },
                    },
                });
            }
            
            // Model definition
            if line.trim().starts_with("model ") && line.contains(&word) {
                let col = line.find(&word)?;
                return Some(Location {
                    uri: Url::parse("file:///").ok()?,
                    range: Range {
                        start: Position { line: i as u32, character: col as u32 },
                        end: Position { line: i as u32, character: (col + word.len()) as u32 },
                    },
                });
            }
            
            // Action definition
            if line.trim().starts_with("action ") && line.contains(&word) {
                let col = line.find(&word)?;
                return Some(Location {
                    uri: Url::parse("file:///").ok()?,
                    range: Range {
                        start: Position { line: i as u32, character: col as u32 },
                        end: Position { line: i as u32, character: (col + word.len()) as u32 },
                    },
                });
            }
        }
        
        None
    }

    /// Find all references to a symbol
    fn find_references(&self, text: &str, position: Position) -> Vec<Location> {
        let mut refs = Vec::new();
        let word = match get_word_at_position(text, position) {
            Some(w) => w,
            None => return refs,
        };
        
        let lines: Vec<&str> = text.lines().collect();
        
        for (i, line) in lines.iter().enumerate() {
            let mut search_start = 0;
            while let Some(col) = line[search_start..].find(&word) {
                let actual_col = search_start + col;
                // Check it's a whole word
                let before_ok = actual_col == 0 || 
                    !line.chars().nth(actual_col - 1).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                let after_ok = actual_col + word.len() >= line.len() ||
                    !line.chars().nth(actual_col + word.len()).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                
                if before_ok && after_ok {
                    refs.push(Location {
                        uri: Url::parse("file:///").unwrap(),
                        range: Range {
                            start: Position { line: i as u32, character: actual_col as u32 },
                            end: Position { line: i as u32, character: (actual_col + word.len()) as u32 },
                        },
                    });
                }
                search_start = actual_col + word.len();
            }
        }
        
        refs
    }

    /// Get completions at a position
    fn get_completions(&self, _uri: &Url, position: Position, text: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        let lines: Vec<&str> = text.lines().collect();
        
        // Check context for smarter completions
        let current_line = lines.get(position.line as usize).unwrap_or(&"");
        let before_cursor = if (position.character as usize) <= current_line.len() {
            &current_line[..position.character as usize]
        } else {
            current_line
        };
        
        // Inside a transition arrow - suggest states
        if before_cursor.contains("->") || before_cursor.ends_with("-[") {
            // Extract states from the document
            for line in &lines {
                if line.contains("states") && line.contains('{') {
                    if let Some(content) = line.split('{').nth(1) {
                        for state in content.split([',', '}']).filter(|s| !s.trim().is_empty()) {
                            let state_name = state.trim();
                            items.push(CompletionItem {
                                label: state_name.to_string(),
                                kind: Some(CompletionItemKind::ENUM_MEMBER),
                                detail: Some("State".to_string()),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
            return items;
        }
        
        // Top-level keywords
        items.push(CompletionItem {
            label: "model".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define a state machine model".to_string()),
            insert_text: Some("model ${1:name} {\n  states { ${2:s0} }\n  initial { ${2:s0} }\n  transitions {\n    ${3}\n  }\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        items.push(CompletionItem {
            label: "rule".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define a rule with a formula".to_string()),
            insert_text: Some("rule {\n  starting_at ${1:$PARENT}\n  formula {\n    ${2:always(true)}\n  }\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        items.push(CompletionItem {
            label: "export default rule".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define an exported default rule".to_string()),
            insert_text: Some("export default rule {\n  starting_at ${1:$PARENT}\n  formula {\n    ${2:always(true)}\n  }\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        items.push(CompletionItem {
            label: "formula".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define a formula".to_string()),
            insert_text: Some("formula {\n  ${1}\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        items.push(CompletionItem {
            label: "action".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define an action".to_string()),
            insert_text: Some("action ${1:NAME} {\n  ${2}\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        items.push(CompletionItem {
            label: "test".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define a test".to_string()),
            insert_text: Some("test \"${1:description}\" {\n  ${2}\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        // Model structure keywords
        items.push(CompletionItem {
            label: "states".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Declare states".to_string()),
            insert_text: Some("states { ${1:s0} }".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        items.push(CompletionItem {
            label: "initial".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Declare initial state".to_string()),
            insert_text: Some("initial { ${1:s0} }".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        items.push(CompletionItem {
            label: "transitions".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define transitions".to_string()),
            insert_text: Some("transitions {\n  ${1:s0} -[${2:ACTION}]-> ${3:s1}\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        // Temporal operators
        items.push(CompletionItem {
            label: "always".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("Always/globally operator (□)".to_string()),
            insert_text: Some("always(${1:formula})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        items.push(CompletionItem {
            label: "eventually".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("Eventually/finally operator (◇)".to_string()),
            insert_text: Some("eventually(${1:formula})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        items.push(CompletionItem {
            label: "until".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("Until operator".to_string()),
            insert_text: Some("until(${1:pre}, ${2:post})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        // Modal operators
        items.push(CompletionItem {
            label: "[<+action>]".to_string(),
            kind: Some(CompletionItemKind::OPERATOR),
            detail: Some("DiamondBox - committed to action".to_string()),
            insert_text: Some("[<+${1:action}>] ${2:formula}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        items.push(CompletionItem {
            label: "<+action>".to_string(),
            kind: Some(CompletionItemKind::OPERATOR),
            detail: Some("Diamond - can perform action".to_string()),
            insert_text: Some("<+${1:action}> ${2:formula}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        items.push(CompletionItem {
            label: "[+action]".to_string(),
            kind: Some(CompletionItemKind::OPERATOR),
            detail: Some("Box - after all action transitions".to_string()),
            insert_text: Some("[+${1:action}] ${2:formula}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        // Fixed points
        items.push(CompletionItem {
            label: "lfp".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("Least fixed point (μ)".to_string()),
            insert_text: Some("lfp(${1:X}, ${2:formula})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        items.push(CompletionItem {
            label: "gfp".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("Greatest fixed point (ν)".to_string()),
            insert_text: Some("gfp(${1:X}, ${2:formula})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        // Predicates
        items.push(CompletionItem {
            label: "signed_by".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("Signature verification predicate".to_string()),
            insert_text: Some("signed_by(${1:/path/to/identity.id})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        items.push(CompletionItem {
            label: "threshold".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("n-of-m multisig predicate".to_string()),
            insert_text: Some("threshold(${1:2}, [${2:/path/to/id1.id}, ${3:/path/to/id2.id}])".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        items.push(CompletionItem {
            label: "oracle_attests".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("Oracle attestation predicate".to_string()),
            insert_text: Some("oracle_attests(${1:/path/to/oracle.id}, ${2:statement})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        items.push(CompletionItem {
            label: "before".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("Timestamp before predicate".to_string()),
            insert_text: Some("before(${1:timestamp})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        items.push(CompletionItem {
            label: "after".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("Timestamp after predicate".to_string()),
            insert_text: Some("after(${1:timestamp})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
        
        items
    }

    /// Get hover information at a position
    fn get_hover(&self, text: &str, position: Position) -> Option<Hover> {
        let word = get_word_at_position(text, position)?;
        
        let contents = match word.as_str() {
            "model" => "**model**\n\nDefines a labeled transition system (state machine).\n\n```modality\nmodel name {\n  states { s0, s1 }\n  initial { s0 }\n  transitions {\n    s0 -[ACTION]-> s1\n  }\n}\n```",
            "rule" => "**rule**\n\nDefines a temporal logic formula that must hold.\n\n```modality\nrule {\n  starting_at $PARENT\n  formula {\n    always([<+signed_by(/id)>] true)\n  }\n}\n```",
            "formula" => "**formula**\n\nA modal mu-calculus formula for verification.",
            "action" => "**action**\n\nDefines a named action with properties.\n\n```modality\naction DEPOSIT {\n  amount: num\n  from: id\n}\n```",
            "test" => "**test**\n\nDefines a test case for model checking.\n\n```modality\ntest \"should reach done\" {\n  assert eventually(done)\n}\n```",
            "always" => "**always(φ)** — □φ\n\nThe formula φ must hold in all future states.\n\nEquivalent to: `gfp(X, []X & φ)`",
            "eventually" => "**eventually(φ)** — ◇φ\n\nThe formula φ must hold in some future state.\n\nEquivalent to: `lfp(X, <>X | φ)`",
            "until" => "**until(p, q)**\n\np must hold until q becomes true.\n\nEquivalent to: `lfp(X, q | (p & <>X))`",
            "lfp" => "**lfp(X, φ)** — μX.φ\n\nLeast fixed point. Used for reachability properties.",
            "gfp" => "**gfp(X, φ)** — νX.φ\n\nGreatest fixed point. Used for safety/invariant properties.",
            "signed_by" => "**signed_by(path)**\n\nVerifies that the commit is signed by the identity at the given path.\n\n```modality\n+signed_by(/users/alice.id)\n```",
            "threshold" => "**threshold(n, [ids...])**\n\nn-of-m multisig. Requires n unique signatures from the list.\n\n```modality\n+threshold(2, [/users/alice.id, /users/bob.id, /users/carol.id])\n```",
            "oracle_attests" => "**oracle_attests(oracle, statement)**\n\nVerifies an oracle has attested to a statement.\n\n```modality\n+oracle_attests(/oracles/price.id, \"BTC > 50000\")\n```",
            "before" => "**before(timestamp)**\n\nMust be executed before the given timestamp.",
            "after" => "**after(timestamp)**\n\nMust be executed after the given timestamp.",
            "states" => "**states**\n\nDeclares the set of states in a model.\n\n```modality\nstates { idle, active, done }\n```",
            "initial" => "**initial**\n\nDeclares the initial state(s) of a model.\n\n```modality\ninitial { idle }\n```",
            "transitions" => "**transitions**\n\nDefines state transitions with actions.\n\n```modality\ntransitions {\n  idle -[START]-> active\n  active -[FINISH]-> done\n}\n```",
            "starting_at" => "**starting_at**\n\nSpecifies the commit hash where the rule takes effect.\n\nUse `$PARENT` to reference the parent commit.",
            "true" => "**true**\n\nBoolean constant, always satisfied.",
            "false" => "**false**\n\nBoolean constant, never satisfied.",
            _ => {
                // Check if it's a state in this document
                if text.contains(&format!("states")) && text.contains(&word) {
                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!("**{}** — State", word),
                        }),
                        range: None,
                    });
                }
                return None;
            }
        };
        
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: contents.to_string(),
            }),
            range: None,
        })
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for ModalityLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        ".".to_string(),
                        "/".to_string(),
                        "<".to_string(),
                        "[".to_string(),
                        "+".to_string(),
                        "-".to_string(),
                    ]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: get_legend(),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: Some(false),
                            ..Default::default()
                        },
                    ),
                ),
                document_formatting_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![
                            CodeActionKind::QUICKFIX,
                            CodeActionKind::REFACTOR,
                        ]),
                        ..Default::default()
                    },
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "modality-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Modality LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let text = params.text_document.text.clone();
        let version = params.text_document.version;
        
        let symbols = self.extract_symbols(&text);
        
        self.documents.insert(
            uri.clone(),
            Document {
                content: Rope::from_str(&text),
                version,
                symbols,
            },
        );
        
        let diagnostics = self.diagnose(&params.text_document.uri, &text).await;
        self.client
            .publish_diagnostics(params.text_document.uri, diagnostics, Some(version))
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let version = params.text_document.version;
        
        if let Some(change) = params.content_changes.into_iter().next() {
            let text = change.text.clone();
            let symbols = self.extract_symbols(&text);
            
            self.documents.insert(
                uri.clone(),
                Document {
                    content: Rope::from_str(&text),
                    version,
                    symbols,
                },
            );
            
            let diagnostics = self.diagnose(&params.text_document.uri, &text).await;
            self.client
                .publish_diagnostics(params.text_document.uri, diagnostics, Some(version))
                .await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri.to_string());
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        
        if let Some(doc) = self.documents.get(&uri) {
            let text = doc.content.to_string();
            let items = self.get_completions(
                &params.text_document_position.text_document.uri,
                params.text_document_position.position,
                &text,
            );
            return Ok(Some(CompletionResponse::Array(items)));
        }
        
        Ok(None)
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri.to_string();
        
        if let Some(doc) = self.documents.get(&uri) {
            let text = doc.content.to_string();
            return Ok(self.get_hover(&text, params.text_document_position_params.position));
        }
        
        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri_str = params.text_document_position_params.text_document.uri.to_string();
        let uri = params.text_document_position_params.text_document.uri.clone();
        
        if let Some(doc) = self.documents.get(&uri_str) {
            let text = doc.content.to_string();
            if let Some(mut location) = self.find_definition(&text, params.text_document_position_params.position) {
                location.uri = uri;
                return Ok(Some(GotoDefinitionResponse::Scalar(location)));
            }
        }
        
        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri_str = params.text_document_position.text_document.uri.to_string();
        let uri = params.text_document_position.text_document.uri.clone();
        
        if let Some(doc) = self.documents.get(&uri_str) {
            let text = doc.content.to_string();
            let mut refs = self.find_references(&text, params.text_document_position.position);
            for r in &mut refs {
                r.uri = uri.clone();
            }
            if !refs.is_empty() {
                return Ok(Some(refs));
            }
        }
        
        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri.to_string();
        
        if let Some(doc) = self.documents.get(&uri) {
            let symbols: Vec<DocumentSymbol> = doc.symbols.iter().map(symbol_info_to_document_symbol).collect();
            return Ok(Some(DocumentSymbolResponse::Nested(symbols)));
        }
        
        Ok(None)
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri.to_string();
        
        if let Some(doc) = self.documents.get(&uri) {
            let text = doc.content.to_string();
            let tokens = compute_semantic_tokens(&text);
            let delta_tokens = tokens_to_lsp(tokens);
            
            // Convert to LSP SemanticToken structs
            let data: Vec<tower_lsp::lsp_types::SemanticToken> = delta_tokens
                .into_iter()
                .map(|t| tower_lsp::lsp_types::SemanticToken {
                    delta_line: t.line,
                    delta_start: t.start,
                    length: t.length,
                    token_type: t.token_type,
                    token_modifiers_bitset: t.modifiers,
                })
                .collect();
            
            return Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
                result_id: None,
                data,
            })));
        }
        
        Ok(None)
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri.to_string();
        
        if let Some(doc) = self.documents.get(&uri) {
            let text = doc.content.to_string();
            let edits = format_document(&text, &params.options);
            return Ok(Some(edits));
        }
        
        Ok(None)
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> Result<Option<CodeActionResponse>> {
        let uri_str = params.text_document.uri.to_string();
        
        if let Some(doc) = self.documents.get(&uri_str) {
            let text = doc.content.to_string();
            let actions = compute_code_actions(
                &text,
                params.range,
                &params.context.diagnostics,
                &params.text_document.uri,
            );
            
            if !actions.is_empty() {
                return Ok(Some(actions));
            }
        }
        
        Ok(None)
    }
}

/// Convert internal symbol info to LSP DocumentSymbol
fn symbol_info_to_document_symbol(info: &DocumentSymbolInfo) -> DocumentSymbol {
    #[allow(deprecated)]
    DocumentSymbol {
        name: info.name.clone(),
        detail: None,
        kind: info.kind,
        tags: None,
        deprecated: None,
        range: info.range,
        selection_range: info.selection_range,
        children: if info.children.is_empty() {
            None
        } else {
            Some(info.children.iter().map(symbol_info_to_document_symbol).collect())
        },
    }
}

/// Find the end of a brace-delimited block
fn find_block_end(lines: &[&str], start: usize) -> Option<usize> {
    let mut depth = 0;
    let mut found_open = false;
    
    for i in start..lines.len() {
        for c in lines[i].chars() {
            if c == '{' {
                depth += 1;
                found_open = true;
            } else if c == '}' {
                depth -= 1;
                if found_open && depth == 0 {
                    return Some(i);
                }
            }
        }
    }
    
    None
}

/// Extract action name from a transition line
fn extract_transition_action(line: &str) -> Option<String> {
    let start = line.find("-[")?;
    let end = line.find("]->")?;
    let action = &line[start + 2..end];
    Some(action.trim().to_string())
}

/// Convert a parse error to an LSP range
fn error_to_range(error: &str, text: &str) -> Range {
    let lines: Vec<&str> = text.lines().collect();
    
    if let Some(line_num) = extract_line_number(error) {
        let line = (line_num.saturating_sub(1)) as u32;
        let line_len = lines.get(line as usize).map(|l| l.len()).unwrap_or(0) as u32;
        return Range {
            start: Position { line, character: 0 },
            end: Position { line, character: line_len },
        };
    }
    
    Range {
        start: Position { line: 0, character: 0 },
        end: Position { line: 0, character: lines.first().map(|l| l.len()).unwrap_or(0) as u32 },
    }
}

/// Try to extract a line number from an error message
fn extract_line_number(error: &str) -> Option<usize> {
    if let Some(idx) = error.find("line ") {
        let rest = &error[idx + 5..];
        let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        return num_str.parse().ok();
    }
    
    for word in error.split_whitespace() {
        if let Some(colon_idx) = word.find(':') {
            let line_part = &word[..colon_idx];
            if let Ok(n) = line_part.parse::<usize>() {
                if n > 0 && n < 10000 {
                    return Some(n);
                }
            }
        }
    }
    
    None
}

/// Get the word at a given position in the text
fn get_word_at_position(text: &str, position: Position) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let line = lines.get(position.line as usize)?;
    let char_idx = position.character as usize;
    
    if char_idx > line.len() {
        return None;
    }
    
    let before: String = line[..char_idx]
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    
    let after: String = line[char_idx..]
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    
    let word = format!("{}{}", before, after);
    if word.is_empty() {
        None
    } else {
        Some(word)
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| ModalityLanguageServer::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
