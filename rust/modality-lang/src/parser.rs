use std::fs;
use std::path::Path;
use crate::ast::{Model, Graph, Transition, Property, PropertySign};

/// Parse a .modality file and return a Model
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Model, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    parse_content(&content)
}

/// Parse the content of a .modality file
pub fn parse_content(content: &str) -> Result<Model, String> {
    let lines: Vec<&str> = content.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .collect();

    if lines.is_empty() {
        return Err("No content found in file".to_string());
    }

    let mut i = 0;
    let mut models = Vec::new();

    while i < lines.len() {
        let line = lines[i];

        if line.starts_with("model ") {
            let (model, new_i) = parse_model(&lines, i)?;
            models.push(model);
            i = new_i;
        } else {
            i += 1;
        }
    }

    if models.is_empty() {
        return Err("No models found in file".to_string());
    }

    // For now, return the first model
    // TODO: Support multiple models
    Ok(models.remove(0))
}

fn parse_model(lines: &[&str], start: usize) -> Result<(Model, usize), String> {
    if start >= lines.len() {
        return Err("Unexpected end of file".to_string());
    }

    let model_line = lines[start];
    let model_name = model_line
        .strip_prefix("model ")
        .and_then(|s| s.strip_suffix(':'))
        .ok_or_else(|| format!("Invalid model declaration: {}", model_line))?;

    let mut model = Model::new(model_name.to_string());
    let mut i = start + 1;

    while i < lines.len() {
        let line = lines[i];

        if line.starts_with("model ") {
            break; // Start of new model
        }

        if line.starts_with("graph ") {
            let (graph, new_i) = parse_graph(lines, i)?;
            model.add_graph(graph);
            i = new_i;
        } else {
            i += 1; // Skip unknown lines
        }
    }

    Ok((model, i))
}

fn parse_graph(lines: &[&str], start: usize) -> Result<(Graph, usize), String> {
    if start >= lines.len() {
        return Err("Unexpected end of file".to_string());
    }

    let graph_line = lines[start];
    let graph_name = graph_line
        .strip_prefix("graph ")
        .and_then(|s| s.strip_suffix(':'))
        .ok_or_else(|| format!("Invalid graph declaration: {}", graph_line))?;
    let mut graph = Graph::new(graph_name.to_string());
    let mut i = start + 1;

    while i < lines.len() {
        let line = lines[i];

        if line.starts_with("model ") || line.starts_with("graph ") {
            break; // Start of new model or graph
        }

        if line.contains("-->") {
            let transition = parse_transition(line)?;
            graph.add_transition(transition);
        }

        i += 1;
    }

    Ok((graph, i))
}

fn parse_transition(line: &str) -> Result<Transition, String> {
    // Parse transition: "n1 --> n2: +blue -red" or "n1 --> n2"
    let parts: Vec<&str> = line.split("-->").collect();
    if parts.len() != 2 {
        return Err(format!("Invalid transition format: {}", line));
    }

    let from = parts[0].trim();
    let to_and_props = parts[1].trim();

    // Check if there's a colon to separate destination from properties
    if to_and_props.contains(':') {
        // Split on colon to separate destination from properties
        let colon_parts: Vec<&str> = to_and_props.split(':').collect();
        if colon_parts.len() != 2 {
            return Err(format!("Invalid transition format (invalid colon): {}", line));
        }

        let to = colon_parts[0].trim();
        let props_str = colon_parts[1].trim();

        let mut transition = Transition::new(from.to_string(), to.to_string());

        // Parse properties
        if !props_str.is_empty() {
            let properties = parse_properties(props_str)?;
            for prop in properties {
                transition.add_property(prop);
            }
        }

        Ok(transition)
    } else {
        // No properties, just destination
        let to = to_and_props;
        Ok(Transition::new(from.to_string(), to.to_string()))
    }
}

fn parse_properties(props_str: &str) -> Result<Vec<Property>, String> {
    let mut properties = Vec::new();

    for prop_str in props_str.split_whitespace() {
        let prop = parse_property(prop_str)?;
        properties.push(prop);
    }

    Ok(properties)
}

fn parse_property(prop_str: &str) -> Result<Property, String> {
    if prop_str.is_empty() {
        return Err("Empty property".to_string());
    }

    let sign = match prop_str.chars().next() {
        Some('+') => PropertySign::Plus,
        Some('-') => PropertySign::Minus,
        _ => return Err(format!("Invalid property sign in: {}", prop_str)),
    };

    let name = prop_str[1..].to_string();
    if name.is_empty() {
        return Err(format!("Property name is empty: {}", prop_str));
    }

    Ok(Property::new(sign, name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::PropertySign;

    #[test]
    fn test_parse_simple_model() {
        let content = r#"
model InitialModel:
  graph g1:
    n1 --> n1
"#;

        let model = parse_content(content).unwrap();

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
    fn test_parse_model_with_properties() {
        let content = r#"
model Model3:
  graph g1:
    n1 --> n2: +blue
    n2 --> n3: +blue
"#;

        let model = parse_content(content).unwrap();

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
    fn test_parse_model_with_multiple_properties() {
        let content = r#"
model Model4:
  graph g1:
    n1 --> n2: +blue -red
    n2 --> n3: +blue -green
    n3 --> n1: -blue +red
"#;

        let model = parse_content(content).unwrap();

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
    fn test_parse_model_with_multiple_graphs() {
        let content = r#"
model Model4:
  graph g1:
    n1 --> n2: +blue -red
    n2 --> n3: +blue -green
    n3 --> n1: -blue +red
  graph g2:
    n1 --> n1: +yellow
"#;

        let model = parse_content(content).unwrap();

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