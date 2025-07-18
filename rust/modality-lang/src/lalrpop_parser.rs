use std::fs;
use std::path::Path;
use crate::ast::Model;
use crate::grammar::ModelParser;

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

    // For now, just parse the first model
    // TODO: Support multiple models
    let parser = ModelParser::new();
    parser.parse(&filtered_content)
        .map_err(|e| format!("Parse error: {:?}", e))
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
    let lines: Vec<&str> = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .collect();

    let mut models = Vec::new();
    let mut i = 0;
    
    while i < lines.len() {
        let line = lines[i];
        
        if line.starts_with("model ") {
            // Find the end of this model
            let mut model_lines = Vec::new();
            model_lines.push(line);
            i += 1;
            
            while i < lines.len() {
                let line = lines[i];
                if line.starts_with("model ") {
                    break; // Start of next model
                }
                model_lines.push(line);
                i += 1;
            }
            
            // Parse this model
            let model_content = model_lines.join("\n");
            let parser = ModelParser::new();
            match parser.parse(&model_content) {
                Ok(model) => models.push(model),
                Err(e) => return Err(format!("Failed to parse model: {:?}", e)),
            }
        } else {
            i += 1;
        }
    }
    
    if models.is_empty() {
        return Err("No models found in file".to_string());
    }
    
    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::PropertySign;

    #[test]
    fn test_parse_simple_model_lalrpop() {
        let content = r#"
model InitialModel:
  graph g1:
    n1 --> n1
"#;
        
        let model = parse_content_lalrpop(content).unwrap();
        
        assert_eq!(model.name, "InitialModel");
        assert_eq!(model.graphs.len(), 1);
        
        let graph = &model.graphs[0];
        assert_eq!(graph.name, "g1");
        assert_eq!(graph.transitions.len(), 1);
        
        let transition = &graph.transitions[0];
        assert_eq!(transition.from, "n1");
        assert_eq!(transition.to, "n1");
        assert_eq!(transition.properties.len(), 0);
    }

    #[test]
    fn test_parse_model_with_properties_lalrpop() {
        let content = r#"
model Model3:
  graph g1:
    n1 --> n2: +blue
    n2 --> n3: +blue
"#;
        
        let model = parse_content_lalrpop(content).unwrap();
        
        assert_eq!(model.name, "Model3");
        assert_eq!(model.graphs.len(), 1);
        
        let graph = &model.graphs[0];
        assert_eq!(graph.name, "g1");
        assert_eq!(graph.transitions.len(), 2);
        
        let transition1 = &graph.transitions[0];
        assert_eq!(transition1.from, "n1");
        assert_eq!(transition1.to, "n2");
        assert_eq!(transition1.properties.len(), 1);
        assert_eq!(transition1.properties[0].sign, PropertySign::Plus);
        assert_eq!(transition1.properties[0].name, "blue");
        
        let transition2 = &graph.transitions[1];
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
  graph g1:
    n1 --> n2: +blue -red
    n2 --> n3: +blue -green
    n3 --> n1: -blue +red
"#;
        
        let model = parse_content_lalrpop(content).unwrap();
        
        assert_eq!(model.name, "Model4");
        assert_eq!(model.graphs.len(), 1);
        
        let graph = &model.graphs[0];
        assert_eq!(graph.name, "g1");
        assert_eq!(graph.transitions.len(), 3);
        
        let transition1 = &graph.transitions[0];
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
  graph g1:
    n1 --> n2: +blue -red
    n2 --> n3: +blue -green
    n3 --> n1: -blue +red
  graph g2:
    n1 --> n1: +yellow
"#;
        
        let model = parse_content_lalrpop(content).unwrap();
        
        assert_eq!(model.name, "Model4");
        assert_eq!(model.graphs.len(), 2);
        
        let graph1 = &model.graphs[0];
        assert_eq!(graph1.name, "g1");
        assert_eq!(graph1.transitions.len(), 3);
        
        let graph2 = &model.graphs[1];
        assert_eq!(graph2.name, "g2");
        assert_eq!(graph2.transitions.len(), 1);
        
        let transition = &graph2.transitions[0];
        assert_eq!(transition.from, "n1");
        assert_eq!(transition.to, "n1");
        assert_eq!(transition.properties.len(), 1);
        assert_eq!(transition.properties[0].sign, PropertySign::Plus);
        assert_eq!(transition.properties[0].name, "yellow");
    }
} 