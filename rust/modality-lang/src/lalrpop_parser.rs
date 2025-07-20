use std::fs;
use std::path::Path;
use crate::ast::{Model, Formula};
use crate::grammar::{TopLevelParser, ModelParser, FormulaParser};

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
    use crate::ast::{PropertySign, FormulaExpr};

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
    fn test_parse_model_with_multiple_graphs_lalrpop() {
        let content = r#"
model Model4:
  part g1:
    n1 --> n2: +blue -red
    n2 --> n3: +blue -green
    n3 --> n1: -blue +red
  part g2:
    n1 --> n1: +yellow
"#;
        
        let model = parse_content_lalrpop(content).unwrap();
        
        assert_eq!(model.name, "Model4");
        assert_eq!(model.parts.len(), 2);
        
        let part1 = &model.parts[0];
        assert_eq!(part1.name, "g1");
        assert_eq!(part1.transitions.len(), 3);
        
        let part2 = &model.parts[1];
        assert_eq!(part2.name, "g2");
        assert_eq!(part2.transitions.len(), 1);
        
        let transition = &part2.transitions[0];
        assert_eq!(transition.from, "n1");
        assert_eq!(transition.to, "n1");
        assert_eq!(transition.properties.len(), 1);
        assert_eq!(transition.properties[0].sign, PropertySign::Plus);
        assert_eq!(transition.properties[0].name, "yellow");
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
        
        let formula1 = &formulas[0];
        assert_eq!(formula1.name, "FormulaTrue");
        assert!(matches!(formula1.expression, FormulaExpr::True));
        
        let formula2 = &formulas[1];
        assert_eq!(formula2.name, "FormulaFalse");
        assert!(matches!(formula2.expression, FormulaExpr::False));
        
        let formula3 = &formulas[2];
        assert_eq!(formula3.name, "FormulaBooleanWff");
        assert!(matches!(formula3.expression, FormulaExpr::And(_, _)));
    }

    #[test]
    fn test_parse_modal_formulas() {
        let content = r#"
formula FormulaDiamondBlueTrue: <+blue> true
formula FormulaBoxNegBlueFalse: [-blue] false
formula FormulaBoxNegBlueTrue: <+blue> <+blue> [-blue] false
"#;
        
        let formulas = parse_all_formulas_content_lalrpop(content).unwrap();
        
        assert_eq!(formulas.len(), 3);
        
        let formula1 = &formulas[0];
        assert_eq!(formula1.name, "FormulaDiamondBlueTrue");
        assert!(matches!(formula1.expression, FormulaExpr::Diamond(_, _)));
        
        let formula2 = &formulas[1];
        assert_eq!(formula2.name, "FormulaBoxNegBlueFalse");
        assert!(matches!(formula2.expression, FormulaExpr::Box(_, _)));
        
        let formula3 = &formulas[2];
        assert_eq!(formula3.name, "FormulaBoxNegBlueTrue");
        assert!(matches!(formula3.expression, FormulaExpr::Diamond(_, _)));
    }
} 