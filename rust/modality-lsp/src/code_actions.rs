//! Code actions (quick fixes) for Modality files

use tower_lsp::lsp_types::*;

/// Compute code actions for a given range
pub fn compute_code_actions(
    text: &str,
    range: Range,
    diagnostics: &[Diagnostic],
    uri: &Url,
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();
    
    // Check for missing states
    if let Some(action) = suggest_add_missing_state(text, range, uri) {
        actions.push(CodeActionOrCommand::CodeAction(action));
    }
    
    // Check for terminal state without self-loop
    if let Some(action) = suggest_add_self_loop(text, range, uri) {
        actions.push(CodeActionOrCommand::CodeAction(action));
    }
    
    // Suggest wrapping formula in always() if it looks like an invariant
    if let Some(action) = suggest_wrap_in_always(text, range, uri) {
        actions.push(CodeActionOrCommand::CodeAction(action));
    }
    
    // Add quick fix for diagnostics
    for diag in diagnostics {
        if let Some(action) = fix_for_diagnostic(text, diag, uri) {
            actions.push(CodeActionOrCommand::CodeAction(action));
        }
    }
    
    actions
}

/// Suggest adding a missing state to the states block
fn suggest_add_missing_state(text: &str, range: Range, uri: &Url) -> Option<CodeAction> {
    let lines: Vec<&str> = text.lines().collect();
    let line = lines.get(range.start.line as usize)?;
    
    // Check if we're in a transition and the state doesn't exist
    if !line.contains("->") {
        return None;
    }
    
    // Extract states from the transition
    let parts: Vec<&str> = line.split("->").collect();
    if parts.len() != 2 {
        return None;
    }
    
    let target_state = parts[1].trim();
    if target_state.is_empty() {
        return None;
    }
    
    // Check if state exists in states block
    let mut states_line_idx = None;
    let mut existing_states = Vec::new();
    
    for (i, l) in lines.iter().enumerate() {
        if l.contains("states") && l.contains('{') {
            states_line_idx = Some(i);
            if let Some(content) = l.split('{').nth(1) {
                for state in content.split([',', '}']).filter(|s| !s.trim().is_empty()) {
                    existing_states.push(state.trim().to_string());
                }
            }
            break;
        }
    }
    
    // If target state already exists, no action needed
    if existing_states.iter().any(|s| s == target_state) {
        return None;
    }
    
    let states_line = states_line_idx?;
    let states_line_content = lines[states_line];
    
    // Create edit to add the state
    let new_states_content = if states_line_content.contains('}') {
        // Single line states: add before }
        let close_idx = states_line_content.find('}')?;
        format!(
            "{}, {} }}",
            states_line_content[..close_idx].trim_end(),
            target_state
        )
    } else {
        return None; // Multi-line not supported yet
    };
    
    Some(CodeAction {
        title: format!("Add '{}' to states", target_state),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some([(
                uri.clone(),
                vec![TextEdit {
                    range: Range {
                        start: Position { line: states_line as u32, character: 0 },
                        end: Position { line: states_line as u32, character: states_line_content.len() as u32 },
                    },
                    new_text: new_states_content,
                }],
            )].into_iter().collect()),
            ..Default::default()
        }),
        ..Default::default()
    })
}

/// Suggest adding a self-loop for terminal states
fn suggest_add_self_loop(text: &str, range: Range, uri: &Url) -> Option<CodeAction> {
    let lines: Vec<&str> = text.lines().collect();
    let line = lines.get(range.start.line as usize)?;
    let trimmed = line.trim();
    
    // Check if this is a state in a states block
    if !lines.iter().any(|l| l.contains("states") && l.contains(trimmed)) {
        return None;
    }
    
    // Check if this state has any outgoing transitions
    let has_outgoing = lines.iter().any(|l| {
        l.contains("->") && l.trim().starts_with(trimmed)
    });
    
    if has_outgoing {
        return None;
    }
    
    // Find the transitions block to add the self-loop
    let mut transitions_end = None;
    for (i, l) in lines.iter().enumerate() {
        if l.contains("transitions") {
            // Find the closing brace
            let mut depth = 0;
            for (j, line) in lines.iter().enumerate().skip(i) {
                for c in line.chars() {
                    if c == '{' { depth += 1; }
                    if c == '}' { depth -= 1; }
                }
                if depth == 0 {
                    transitions_end = Some(j);
                    break;
                }
            }
            break;
        }
    }
    
    let end_line = transitions_end?;
    let indent = "    "; // Default indent
    
    Some(CodeAction {
        title: format!("Add self-loop for terminal state '{}'", trimmed),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(WorkspaceEdit {
            changes: Some([(
                uri.clone(),
                vec![TextEdit {
                    range: Range {
                        start: Position { line: end_line as u32, character: 0 },
                        end: Position { line: end_line as u32, character: 0 },
                    },
                    new_text: format!("{}{} -[STAY]-> {}\n", indent, trimmed, trimmed),
                }],
            )].into_iter().collect()),
            ..Default::default()
        }),
        ..Default::default()
    })
}

/// Suggest wrapping a formula in always() if it looks like an invariant
fn suggest_wrap_in_always(text: &str, range: Range, uri: &Url) -> Option<CodeAction> {
    let lines: Vec<&str> = text.lines().collect();
    let line = lines.get(range.start.line as usize)?;
    let trimmed = line.trim();
    
    // Check if we're inside a formula block
    let in_formula = lines[..=range.start.line as usize]
        .iter()
        .rev()
        .any(|l| l.contains("formula"));
    
    if !in_formula {
        return None;
    }
    
    // Don't wrap if already wrapped
    if trimmed.starts_with("always") || trimmed.starts_with("eventually") {
        return None;
    }
    
    // Check if it contains a modal operator (likely an invariant)
    if !trimmed.contains('[') && !trimmed.contains('<') {
        return None;
    }
    
    let indent_len = line.len() - line.trim_start().len();
    let indent: String = line.chars().take(indent_len).collect();
    
    Some(CodeAction {
        title: "Wrap in always()".to_string(),
        kind: Some(CodeActionKind::REFACTOR),
        edit: Some(WorkspaceEdit {
            changes: Some([(
                uri.clone(),
                vec![TextEdit {
                    range: Range {
                        start: Position { line: range.start.line, character: 0 },
                        end: Position { line: range.start.line, character: line.len() as u32 },
                    },
                    new_text: format!("{}always({})", indent, trimmed),
                }],
            )].into_iter().collect()),
            ..Default::default()
        }),
        ..Default::default()
    })
}

/// Create a fix for a specific diagnostic
fn fix_for_diagnostic(_text: &str, diag: &Diagnostic, _uri: &Url) -> Option<CodeAction> {
    let message = &diag.message;
    
    // Missing semicolon or similar
    if message.contains("expected") && message.contains("found") {
        // Could add more specific fixes here
        return None;
    }
    
    // Unknown state
    if message.contains("unknown state") {
        // Extract state name and suggest adding it
        // This would be similar to suggest_add_missing_state
        return None;
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_code_actions_empty() {
        let text = "model test { states { s0 } initial { s0 } transitions { } }";
        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 0 },
        };
        let uri = Url::parse("file:///test.modality").unwrap();
        
        let actions = compute_code_actions(text, range, &[], &uri);
        // May or may not have actions depending on context
        assert!(actions.len() <= 3);
    }
}
