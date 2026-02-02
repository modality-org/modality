use dashmap::DashMap;
use ropey::Rope;
use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use modality_lang::{parse_content_lalrpop, ModelParser, FormulaParser};

/// Document state stored for each open file
struct Document {
    content: Rope,
    version: i32,
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

    /// Get completions at a position
    fn get_completions(&self, _uri: &Url, _position: Position, text: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
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
        
        items
    }

    /// Get hover information at a position
    fn get_hover(&self, text: &str, position: Position) -> Option<Hover> {
        let word = get_word_at_position(text, position)?;
        
        let contents = match word.as_str() {
            "model" => "**model**\n\nDefines a labeled transition system (state machine).\n\n```modality\nmodel name {\n  states { s0, s1 }\n  initial { s0 }\n  transitions {\n    s0 -[ACTION]-> s1\n  }\n}\n```",
            "rule" => "**rule**\n\nDefines a temporal logic formula that must hold.\n\n```modality\nrule {\n  starting_at $PARENT\n  formula {\n    always([<+signed_by(/id)>] true)\n  }\n}\n```",
            "formula" => "**formula**\n\nA modal mu-calculus formula for verification.",
            "always" => "**always(φ)** — □φ\n\nThe formula φ must hold in all future states.\n\nEquivalent to: `gfp(X, []X & φ)`",
            "eventually" => "**eventually(φ)** — ◇φ\n\nThe formula φ must hold in some future state.\n\nEquivalent to: `lfp(X, <>X | φ)`",
            "until" => "**until(p, q)**\n\np must hold until q becomes true.\n\nEquivalent to: `lfp(X, q | (p & <>X))`",
            "lfp" => "**lfp(X, φ)** — μX.φ\n\nLeast fixed point. Used for reachability properties.",
            "gfp" => "**gfp(X, φ)** — νX.φ\n\nGreatest fixed point. Used for safety/invariant properties.",
            "signed_by" => "**signed_by(path)**\n\nVerifies that the commit is signed by the identity at the given path.\n\n```modality\n+signed_by(/users/alice.id)\n```",
            "threshold" => "**threshold(n, [ids...])**\n\nn-of-m multisig. Requires n unique signatures from the list.\n\n```modality\n+threshold(2, [/users/alice.id, /users/bob.id, /users/carol.id])\n```",
            "states" => "**states**\n\nDeclares the set of states in a model.\n\n```modality\nstates { idle, active, done }\n```",
            "initial" => "**initial**\n\nDeclares the initial state(s) of a model.\n\n```modality\ninitial { idle }\n```",
            "transitions" => "**transitions**\n\nDefines state transitions with actions.\n\n```modality\ntransitions {\n  idle -[START]-> active\n  active -[FINISH]-> done\n}\n```",
            "starting_at" => "**starting_at**\n\nSpecifies the commit hash where the rule takes effect.\n\nUse `$PARENT` to reference the parent commit.",
            _ => return None,
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
                    ]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
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
        
        self.documents.insert(
            uri.clone(),
            Document {
                content: Rope::from_str(&text),
                version,
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
            self.documents.insert(
                uri.clone(),
                Document {
                    content: Rope::from_str(&text),
                    version,
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
}

/// Convert a parse error to an LSP range
fn error_to_range(error: &str, text: &str) -> Range {
    // Try to extract line/column from error message
    // Default to first line if we can't parse
    let lines: Vec<&str> = text.lines().collect();
    
    // Look for patterns like "line 5" or "at 5:10"
    if let Some(line_num) = extract_line_number(error) {
        let line = (line_num.saturating_sub(1)) as u32;
        let line_len = lines.get(line as usize).map(|l| l.len()).unwrap_or(0) as u32;
        return Range {
            start: Position { line, character: 0 },
            end: Position { line, character: line_len },
        };
    }
    
    // Default: highlight first line
    Range {
        start: Position { line: 0, character: 0 },
        end: Position { line: 0, character: lines.first().map(|l| l.len()).unwrap_or(0) as u32 },
    }
}

/// Try to extract a line number from an error message
fn extract_line_number(error: &str) -> Option<usize> {
    // Look for "line N" pattern
    if let Some(idx) = error.find("line ") {
        let rest = &error[idx + 5..];
        let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        return num_str.parse().ok();
    }
    
    // Look for "N:M" pattern (line:col)
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
    
    // Find word boundaries
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
