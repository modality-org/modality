//! Code formatter for Modality files

use tower_lsp::lsp_types::*;

/// Format a Modality document
pub fn format_document(text: &str, options: &FormattingOptions) -> Vec<TextEdit> {
    let indent_str = if options.insert_spaces {
        " ".repeat(options.tab_size as usize)
    } else {
        "\t".to_string()
    };
    
    let formatted = format_text(text, &indent_str);
    
    // Return a single edit that replaces the entire document
    let lines: Vec<&str> = text.lines().collect();
    let last_line = lines.len().saturating_sub(1);
    let last_char = lines.last().map(|l| l.len()).unwrap_or(0);
    
    vec![TextEdit {
        range: Range {
            start: Position { line: 0, character: 0 },
            end: Position {
                line: last_line as u32,
                character: last_char as u32,
            },
        },
        new_text: formatted,
    }]
}

/// Format the text with proper indentation
fn format_text(text: &str, indent: &str) -> String {
    let mut result = String::new();
    let mut depth = 0;
    let mut prev_was_blank = false;
    
    for line in text.lines() {
        let trimmed = line.trim();
        
        // Skip multiple consecutive blank lines
        if trimmed.is_empty() {
            if !prev_was_blank && !result.is_empty() {
                result.push('\n');
            }
            prev_was_blank = true;
            continue;
        }
        prev_was_blank = false;
        
        // Decrease indent for closing braces
        if trimmed.starts_with('}') {
            depth = depth.saturating_sub(1);
        }
        
        // Apply indentation
        for _ in 0..depth {
            result.push_str(indent);
        }
        
        // Format the line content
        let formatted_line = format_line(trimmed);
        result.push_str(&formatted_line);
        result.push('\n');
        
        // Increase indent after opening braces
        if trimmed.ends_with('{') {
            depth += 1;
        }
    }
    
    // Remove trailing newline if present, then add exactly one
    result = result.trim_end().to_string();
    result.push('\n');
    
    result
}

/// Format a single line
fn format_line(line: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        let c = chars[i];
        
        // Handle transition arrows: ensure spaces around -[...]->
        if c == '-' && i + 1 < chars.len() && chars[i + 1] == '[' {
            // Ensure space before -[
            if !result.is_empty() && !result.ends_with(' ') {
                result.push(' ');
            }
            
            // Copy the entire transition: -[ACTION]->
            while i < chars.len() {
                result.push(chars[i]);
                if chars[i] == '>' && i > 0 && chars[i - 1] == '-' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            
            // Ensure space after ]->
            if i < chars.len() && chars[i] != ' ' {
                result.push(' ');
            }
            continue;
        }
        
        // Handle braces: ensure space before {
        if c == '{' {
            if !result.is_empty() && !result.ends_with(' ') {
                result.push(' ');
            }
            result.push(c);
            i += 1;
            continue;
        }
        
        // Handle colons: ensure space after
        if c == ':' {
            result.push(c);
            if i + 1 < chars.len() && chars[i + 1] != ' ' {
                result.push(' ');
            }
            i += 1;
            continue;
        }
        
        // Handle commas: ensure space after
        if c == ',' {
            result.push(c);
            if i + 1 < chars.len() && chars[i + 1] != ' ' {
                result.push(' ');
            }
            i += 1;
            continue;
        }
        
        // Collapse multiple spaces
        if c == ' ' {
            if !result.ends_with(' ') {
                result.push(' ');
            }
            i += 1;
            continue;
        }
        
        result.push(c);
        i += 1;
    }
    
    result.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_model() {
        let input = r#"model  escrow{
states{idle,funded,complete}
initial{idle}
transitions{
idle-[DEPOSIT]->funded
funded-[RELEASE]->complete
}
}"#;
        
        let options = FormattingOptions {
            tab_size: 2,
            insert_spaces: true,
            ..Default::default()
        };
        
        let edits = format_document(input, &options);
        assert_eq!(edits.len(), 1);
        
        let formatted = &edits[0].new_text;
        assert!(formatted.contains("states {"));
        assert!(formatted.contains("-[DEPOSIT]->"));
    }
    
    #[test]
    fn test_format_line_transition() {
        let line = "idle-[DEPOSIT]->funded";
        let formatted = format_line(line);
        assert_eq!(formatted, "idle -[DEPOSIT]-> funded");
    }
}
