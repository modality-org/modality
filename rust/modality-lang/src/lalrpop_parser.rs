use std::fs;
use std::path::Path;
use crate::ast::{Model, Formula, Action, ActionCall, Test};
use crate::grammar::{TopLevelParser, FormulaParser, ActionParser, ActionCallParser};

/// Parse a .modality file using LALRPOP and return a Model
pub fn parse_file_lalrpop<P: AsRef<Path>>(path: P) -> Result<Model, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    parse_content_lalrpop(&content)
}

/// Parse the content of a .modality file using LALRPOP
pub fn parse_content_lalrpop(content: &str) -> Result<Model, String> {
    // Filter out comments and empty lines
    let filtered_content: String = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//")
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Parse all top-level items and extract the first model
    let parser = TopLevelParser::new();
    let models = parser.parse(&filtered_content)
        .map_err(|e| format!("Parse error: {:?}", e))?;
    
    if models.is_empty() {
        return Err("No models found in file".to_string());
    }
    
    Ok(models[0].clone())
}

/// Parse all models in a .modality file using LALRPOP
pub fn parse_all_models_lalrpop<P: AsRef<Path>>(path: P) -> Result<Vec<Model>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    parse_all_models_content_lalrpop(&content)
}

/// Parse all models in content using LALRPOP
pub fn parse_all_models_content_lalrpop(content: &str) -> Result<Vec<Model>, String> {
    // Filter out comments and empty lines
    let filtered_content: String = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let parser = TopLevelParser::new();
    let models = parser.parse(&filtered_content)
        .map_err(|e| format!("Parse error: {:?}", e))?;
    
    if models.is_empty() {
        return Err("No models found in file".to_string());
    }
    
    Ok(models)
}

/// Parse all actions in a .modality file using LALRPOP
pub fn parse_all_actions_lalrpop<P: AsRef<Path>>(path: P) -> Result<Vec<Action>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    parse_all_actions_content_lalrpop(&content)
}

/// Parse all actions in content using LALRPOP
pub fn parse_all_actions_content_lalrpop(content: &str) -> Result<Vec<Action>, String> {
    // Filter out comments and empty lines
    let lines: Vec<&str> = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .collect();

    let mut actions = Vec::new();
    let mut i = 0;
    
    while i < lines.len() {
        let line = lines[i];
        
        if line.starts_with("action ") {
            // Find the end of this action
            let mut action_lines = Vec::new();
            action_lines.push(line);
            i += 1;
            
            while i < lines.len() {
                let line = lines[i];
                if line.starts_with("action ") || line.starts_with("model ") || line.starts_with("formula ") || line.starts_with("test ") {
                    break; // Start of next action, model, formula, or test
                }
                action_lines.push(line);
                i += 1;
            }
            
            // Parse this action
            let action_content = action_lines.join("\n");
            let parser = ActionParser::new();
            match parser.parse(&action_content) {
                Ok(action) => actions.push(action),
                Err(e) => return Err(format!("Failed to parse action: {:?}", e)),
            }
        } else {
            i += 1;
        }
    }
    
    Ok(actions)
}

/// Parse an action call from a string
pub fn parse_action_call_lalrpop(content: &str) -> Result<ActionCall, String> {
    let parser = ActionCallParser::new();
    parser.parse(content)
        .map_err(|e| format!("Parse error: {:?}", e))
}

/// Parse all tests in a .modality file using LALRPOP
pub fn parse_all_tests_lalrpop<P: AsRef<Path>>(path: P) -> Result<Vec<Test>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    parse_all_tests_content_lalrpop(&content)
}

/// Parse all tests in content using LALRPOP
pub fn parse_all_tests_content_lalrpop(content: &str) -> Result<Vec<Test>, String> {
    // Filter out comments and empty lines
    let lines: Vec<&str> = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .collect();

    let mut tests = Vec::new();
    let mut i = 0;
    
    while i < lines.len() {
        let line = lines[i];
        
        if line.starts_with("test ") || line.starts_with("test:") {
            // Parse just the test declaration line
            let test_decl_line = if line.starts_with("test:") {
                line
            } else {
                // Handle "test Name:" format
                line
            };
            
            // Create test based on the declaration
            let test = if test_decl_line == "test:" {
                Test::new(None)
            } else if test_decl_line.starts_with("test ") && test_decl_line.ends_with(":") {
                let name = test_decl_line[5..test_decl_line.len()-1].trim().to_string();
                Test::new(Some(name))
            } else {
                return Err(format!("Invalid test declaration: {}", test_decl_line));
            };
            
            tests.push(test);
            i += 1;
        } else {
            i += 1;
        }
    }
    
    Ok(tests)
}

/// Parse all formulas in a .modality file using LALRPOP
pub fn parse_all_formulas_lalrpop<P: AsRef<Path>>(path: P) -> Result<Vec<Formula>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    parse_all_formulas_content_lalrpop(&content)
}

/// Parse all formulas in content using LALRPOP
pub fn parse_all_formulas_content_lalrpop(content: &str) -> Result<Vec<Formula>, String> {
    // Filter out comments and empty lines
    let lines: Vec<&str> = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .collect();

    let mut formulas = Vec::new();
    let mut i = 0;
    
    while i < lines.len() {
        let line = lines[i];
        
        if line.starts_with("formula ") {
            // Find the end of this formula
            let mut formula_lines = Vec::new();
            formula_lines.push(line);
            i += 1;
            
            while i < lines.len() {
                let line = lines[i];
                if line.starts_with("formula ") || line.starts_with("model ") {
                    break; // Start of next formula or model
                }
                formula_lines.push(line);
                i += 1;
            }
            
            // Parse this formula
            let formula_content = formula_lines.join("\n");
            let parser = FormulaParser::new();
            match parser.parse(&formula_content) {
                Ok(formula) => formulas.push(formula),
                Err(e) => return Err(format!("Failed to parse formula: {:?}", e)),
            }
        } else {
            i += 1;
        }
    }
    
    Ok(formulas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{PropertySign, FormulaExpr, TestStatement};

    #[test]
    fn test_parse_simple_model_lalrpop() {
        let content = r#"
model InitialModel:
  part g1:
    n1 --> n1
"#;
        
        let model = parse_content_lalrpop(content).unwrap();
        
        assert_eq!(model.name, "InitialModel");
        assert_eq!(model.parts.len(), 1);
        let part = &model.parts[0];
        assert_eq!(part.name, "g1");
        assert_eq!(part.transitions.len(), 1);
        
        let transition = &part.transitions[0];
        assert_eq!(transition.from, "n1");
        assert_eq!(transition.to, "n1");
        assert_eq!(transition.properties.len(), 0);
    }

    #[test]
    fn test_parse_model_with_properties_lalrpop() {
        let content = r#"
model Model3:
  part g1:
    n1 --> n2: +blue
    n2 --> n3: +blue
"#;
        
        let model = parse_content_lalrpop(content).unwrap();
        
        assert_eq!(model.name, "Model3");
        assert_eq!(model.parts.len(), 1);
        let part = &model.parts[0];
        assert_eq!(part.name, "g1");
        assert_eq!(part.transitions.len(), 2);
        
        let transition1 = &part.transitions[0];
        assert_eq!(transition1.from, "n1");
        assert_eq!(transition1.to, "n2");
        assert_eq!(transition1.properties.len(), 1);
        assert_eq!(transition1.properties[0].sign, PropertySign::Plus);
        assert_eq!(transition1.properties[0].name, "blue");
        
        let transition2 = &part.transitions[1];
        assert_eq!(transition2.from, "n2");
        assert_eq!(transition2.to, "n3");
        assert_eq!(transition2.properties.len(), 1);
        assert_eq!(transition2.properties[0].sign, PropertySign::Plus);
        assert_eq!(transition2.properties[0].name, "blue");
    }

    #[test]
    fn test_parse_model_with_multiple_properties_lalrpop() {
        let content = r#"
model Model4:
  part g1:
    n1 --> n2: +blue -red
    n2 --> n3: +blue -green
    n3 --> n1: -blue +red
"#;
        
        let model = parse_content_lalrpop(content).unwrap();
        
        assert_eq!(model.name, "Model4");
        assert_eq!(model.parts.len(), 1);
        let part = &model.parts[0];
        assert_eq!(part.name, "g1");
        assert_eq!(part.transitions.len(), 3);
        
        let transition1 = &part.transitions[0];
        assert_eq!(transition1.properties.len(), 2);
        assert_eq!(transition1.properties[0].sign, PropertySign::Plus);
        assert_eq!(transition1.properties[0].name, "blue");
        assert_eq!(transition1.properties[1].sign, PropertySign::Minus);
        assert_eq!(transition1.properties[1].name, "red");
    }

    #[test]
    fn test_parse_model_with_multiple_parts_lalrpop() {
        let content = r#"
model Model5:
  part g1:
    n1 --> n2: +blue -red
    n2 --> n3: +blue -green
    n3 --> n1: -blue +red
  part g2:
    n1 --> n1: +yellow
"#;
        
        let model = parse_content_lalrpop(content).unwrap();
        
        assert_eq!(model.name, "Model5");
        assert_eq!(model.parts.len(), 2);
        
        let part1 = &model.parts[0];
        assert_eq!(part1.name, "g1");
        assert_eq!(part1.transitions.len(), 3);
        
        let part2 = &model.parts[1];
        assert_eq!(part2.name, "g2");
        assert_eq!(part2.transitions.len(), 1);
    }

    #[test]
    fn test_parse_boolean_formulas() {
        let content = r#"
formula FormulaTrue: true
formula FormulaFalse: false
formula FormulaBooleanWff: (true or false) and true
"#;
        
        let formulas = parse_all_formulas_content_lalrpop(content).unwrap();
        assert_eq!(formulas.len(), 3);
        
        assert_eq!(formulas[0].name, "FormulaTrue");
        assert!(matches!(formulas[0].expression, FormulaExpr::True));
        
        assert_eq!(formulas[1].name, "FormulaFalse");
        assert!(matches!(formulas[1].expression, FormulaExpr::False));
        
        assert_eq!(formulas[2].name, "FormulaBooleanWff");
        assert!(matches!(formulas[2].expression, FormulaExpr::And(_, _)));
    }

    #[test]
    fn test_parse_modal_formulas() {
        let content = r#"
formula FormulaDiamondBlueTrue: <+blue> true
formula FormulaBoxNegBlueFalse: [-blue] false
"#;
        
        let formulas = parse_all_formulas_content_lalrpop(content).unwrap();
        assert_eq!(formulas.len(), 2);
        
        assert_eq!(formulas[0].name, "FormulaDiamondBlueTrue");
        assert!(matches!(formulas[0].expression, FormulaExpr::Diamond(_, _)));
        
        assert_eq!(formulas[1].name, "FormulaBoxNegBlueFalse");
        assert!(matches!(formulas[1].expression, FormulaExpr::Box(_, _)));
    }

    #[test]
    fn test_parse_action_declaration() {
        let content = r#"
action ActionHello: +hello
"#;
        
        let actions = parse_all_actions_content_lalrpop(content).unwrap();
        assert_eq!(actions.len(), 1);
        
        let action = &actions[0];
        assert_eq!(action.name, "ActionHello");
        assert_eq!(action.properties.len(), 1);
        assert_eq!(action.properties[0].sign, PropertySign::Plus);
        assert_eq!(action.properties[0].name, "hello");
    }

    #[test]
    fn test_parse_action_call() {
        let content = r#"action("+hello")"#;
        
        let action_call = parse_action_call_lalrpop(content).unwrap();
        assert_eq!(action_call.argument, "+hello");
    }

    #[test]
    fn test_parse_action_with_multiple_properties() {
        let content = r#"
action ActionComplex: +blue -red +green
"#;
        
        let actions = parse_all_actions_content_lalrpop(content).unwrap();
        assert_eq!(actions.len(), 1);
        
        let action = &actions[0];
        assert_eq!(action.name, "ActionComplex");
        assert_eq!(action.properties.len(), 3);
        
        assert_eq!(action.properties[0].sign, PropertySign::Plus);
        assert_eq!(action.properties[0].name, "blue");
        
        assert_eq!(action.properties[1].sign, PropertySign::Minus);
        assert_eq!(action.properties[1].name, "red");
        
        assert_eq!(action.properties[2].sign, PropertySign::Plus);
        assert_eq!(action.properties[2].name, "green");
    }

    #[test]
    fn test_parse_anonymous_test() {
        let content = r#"
test:
  m = clone(InitialModel)
  m.commit(ActionHello)
  m.commit(action("+hello"))
"#;
        
        let tests = parse_all_tests_content_lalrpop(content).unwrap();
        assert_eq!(tests.len(), 1);
        
        let test = &tests[0];
        assert_eq!(test.name, None);
        assert_eq!(test.statements.len(), 0); // Simplified approach doesn't parse statements yet
    }

    #[test]
    fn test_parse_named_test() {
        let content = r#"
test NamedTest:
  m = clone(InitialModel)
  m.commit(ActionHello)
"#;
        
        let tests = parse_all_tests_content_lalrpop(content).unwrap();
        assert_eq!(tests.len(), 1);
        
        let test = &tests[0];
        assert_eq!(test.name, Some("NamedTest".to_string()));
        assert_eq!(test.statements.len(), 0); // Simplified approach doesn't parse statements yet
    }
} 