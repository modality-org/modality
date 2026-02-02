//! Semantic tokens for enhanced syntax highlighting

use tower_lsp::lsp_types::*;

/// Semantic token types used by Modality LSP
pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,      // 0: model, rule, formula, states, etc.
    SemanticTokenType::TYPE,         // 1: state names
    SemanticTokenType::FUNCTION,     // 2: predicates (signed_by, threshold, etc.)
    SemanticTokenType::OPERATOR,     // 3: modal operators (<+>, [+], etc.)
    SemanticTokenType::VARIABLE,     // 4: action names
    SemanticTokenType::STRING,       // 5: paths (/path/to/id)
    SemanticTokenType::NUMBER,       // 6: numbers
    SemanticTokenType::COMMENT,      // 7: comments
    SemanticTokenType::MACRO,        // 8: temporal operators (always, eventually, until)
    SemanticTokenType::PARAMETER,    // 9: fixed point variables (X in lfp(X, ...))
];

/// Semantic token modifiers
pub const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,  // 0
    SemanticTokenModifier::DEFINITION,   // 1
    SemanticTokenModifier::READONLY,     // 2
];

/// A semantic token with position and type information
#[derive(Debug, Clone)]
pub struct SemanticToken {
    pub line: u32,
    pub start: u32,
    pub length: u32,
    pub token_type: u32,
    pub modifiers: u32,
}

/// Compute semantic tokens for a document
pub fn compute_semantic_tokens(text: &str) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    
    // Track known states for highlighting
    let mut known_states: Vec<String> = Vec::new();
    
    // First pass: collect state names
    for line in &lines {
        if line.contains("states") && line.contains('{') {
            if let Some(content) = line.split('{').nth(1) {
                for state in content.split([',', '}']).filter(|s| !s.trim().is_empty()) {
                    known_states.push(state.trim().to_string());
                }
            }
        }
    }
    
    for (line_num, line) in lines.iter().enumerate() {
        let line_num = line_num as u32;
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        
        while i < chars.len() {
            // Skip whitespace
            if chars[i].is_whitespace() {
                i += 1;
                continue;
            }
            
            // Comments (// style)
            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
                tokens.push(SemanticToken {
                    line: line_num,
                    start: i as u32,
                    length: (chars.len() - i) as u32,
                    token_type: 7, // COMMENT
                    modifiers: 0,
                });
                break;
            }
            
            // Paths (/path/to/something)
            if chars[i] == '/' && (i == 0 || !chars[i-1].is_alphanumeric()) {
                let start = i;
                i += 1;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '/' || chars[i] == '_' || chars[i] == '.' || chars[i] == '-') {
                    i += 1;
                }
                if i > start + 1 {
                    tokens.push(SemanticToken {
                        line: line_num,
                        start: start as u32,
                        length: (i - start) as u32,
                        token_type: 5, // STRING (path)
                        modifiers: 0,
                    });
                }
                continue;
            }
            
            // Modal operators: [<+...>], <+...>, [+...], [-...], etc.
            if chars[i] == '[' || chars[i] == '<' {
                let start = i;
                let mut depth = 0;
                let open_char = chars[i];
                let close_char = if open_char == '[' { ']' } else { '>' };
                
                loop {
                    if i >= chars.len() {
                        break;
                    }
                    if chars[i] == open_char || chars[i] == '<' || chars[i] == '[' {
                        depth += 1;
                    } else if chars[i] == close_char || chars[i] == '>' || chars[i] == ']' {
                        depth -= 1;
                        if depth <= 0 {
                            i += 1;
                            break;
                        }
                    }
                    i += 1;
                }
                
                let token_str: String = chars[start..i].iter().collect();
                if token_str.contains('+') || token_str.contains('-') {
                    tokens.push(SemanticToken {
                        line: line_num,
                        start: start as u32,
                        length: (i - start) as u32,
                        token_type: 3, // OPERATOR
                        modifiers: 0,
                    });
                }
                continue;
            }
            
            // Transition arrow
            if i + 2 < chars.len() && chars[i] == '-' && chars[i + 1] == '[' {
                // Find the full transition: -[ACTION]->
                let start = i;
                while i < chars.len() && !(chars[i] == '>' && i > 0 && chars[i-1] == '-') {
                    i += 1;
                }
                if i < chars.len() {
                    i += 1;
                }
                
                // Highlight the action name inside
                let token_str: String = chars[start..i].iter().collect();
                if let Some(action_start) = token_str.find('[') {
                    if let Some(action_end) = token_str.find(']') {
                        let action = &token_str[action_start + 1..action_end];
                        if !action.is_empty() {
                            tokens.push(SemanticToken {
                                line: line_num,
                                start: (start + action_start + 1) as u32,
                                length: action.len() as u32,
                                token_type: 4, // VARIABLE (action)
                                modifiers: 0,
                            });
                        }
                    }
                }
                continue;
            }
            
            // Identifiers and keywords
            if chars[i].is_alphabetic() || chars[i] == '_' {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                
                let token_type = match word.as_str() {
                    // Keywords
                    "model" | "rule" | "formula" | "action" | "test" |
                    "states" | "initial" | "transitions" | "starting_at" |
                    "export" | "default" | "true" | "false" => 0, // KEYWORD
                    
                    // Temporal operators
                    "always" | "eventually" | "until" | "lfp" | "gfp" => 8, // MACRO
                    
                    // Predicates
                    "signed_by" | "threshold" | "oracle_attests" | 
                    "before" | "after" | "hash_matches" |
                    "num_eq" | "num_gt" | "num_lt" | "num_gte" | "num_lte" |
                    "text_eq" | "bool_eq" => 2, // FUNCTION
                    
                    // Single uppercase letters are likely fixed-point variables
                    _ if word.len() == 1 && word.chars().next().unwrap().is_uppercase() => 9, // PARAMETER
                    
                    // Check if it's a known state
                    _ if known_states.contains(&word) => 1, // TYPE (state)
                    
                    // All uppercase = action name
                    _ if word.chars().all(|c| c.is_uppercase() || c == '_') && word.len() > 1 => 4, // VARIABLE (action)
                    
                    _ => continue, // Skip unknown identifiers
                };
                
                tokens.push(SemanticToken {
                    line: line_num,
                    start: start as u32,
                    length: (i - start) as u32,
                    token_type,
                    modifiers: 0,
                });
                continue;
            }
            
            // Numbers
            if chars[i].is_ascii_digit() {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '_') {
                    i += 1;
                }
                tokens.push(SemanticToken {
                    line: line_num,
                    start: start as u32,
                    length: (i - start) as u32,
                    token_type: 6, // NUMBER
                    modifiers: 0,
                });
                continue;
            }
            
            // Skip other characters
            i += 1;
        }
    }
    
    tokens
}

/// Convert semantic tokens to the LSP delta format
pub fn tokens_to_lsp(tokens: Vec<SemanticToken>) -> Vec<SemanticToken> {
    // Sort tokens by position
    let mut tokens = tokens;
    tokens.sort_by(|a, b| {
        a.line.cmp(&b.line).then(a.start.cmp(&b.start))
    });
    
    // Convert to delta format
    let mut result = Vec::with_capacity(tokens.len());
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;
    
    for token in tokens {
        let delta_line = token.line - prev_line;
        let delta_start = if delta_line == 0 {
            token.start - prev_start
        } else {
            token.start
        };
        
        result.push(SemanticToken {
            line: delta_line,
            start: delta_start,
            length: token.length,
            token_type: token.token_type,
            modifiers: token.modifiers,
        });
        
        prev_line = token.line;
        prev_start = token.start;
    }
    
    result
}

/// Get the semantic tokens legend for capability registration
pub fn get_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: TOKEN_TYPES.to_vec(),
        token_modifiers: TOKEN_MODIFIERS.to_vec(),
    }
}
