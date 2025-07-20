use modality_lang::PropertySign;

fn main() -> Result<(), String> {
    // Read the file content
    let content = std::fs::read_to_string("examples/models/SimpleExamples.modality")
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // Parse each model manually
    let lines: Vec<&str> = content.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .collect();

    let mut i = 0;
    let mut model_count = 0;

    while i < lines.len() {
        let line = lines[i];

        if line.starts_with("model ") {
            model_count += 1;
            println!("\n=== Model {} ===", model_count);

            // Parse this model
            let (model, new_i) = parse_single_model(&lines, i)?;
            i = new_i;

            println!("Model name: {}", model.name);
            println!("Number of parts: {}", model.parts.len());
            
            for (part_idx, part) in model.parts.iter().enumerate() {
                println!("  Part {}: {}", part_idx + 1, part.name);
                println!("    Transitions: {}", part.transitions.len());

                for transition in &part.transitions {
                    print!("    {} --> {}:", transition.from, transition.to);

                    if transition.properties.is_empty() {
                        println!();
                    } else {
                        for prop in &transition.properties {
                            let sign = match prop.sign {
                                PropertySign::Plus => "+",
                                PropertySign::Minus => "-",
                            };
                            print!(" {}{}", sign, prop.name);
                        }
                        println!();
                    }
                }
            }
        } else {
            i += 1;
        }
    }

    println!("\nTotal models found: {}", model_count);

    Ok(())
}

// Helper function to parse a single model
fn parse_single_model(lines: &[&str], start: usize) -> Result<(modality_lang::Model, usize), String> {
    if start >= lines.len() {
        return Err("Unexpected end of file".to_string());
    }

    let model_line = lines[start];
    let model_name = model_line
        .strip_prefix("model ")
        .and_then(|s| s.strip_suffix(':'))
        .ok_or_else(|| format!("Invalid model declaration: {}", model_line))?;

    let mut model = modality_lang::Model::new(model_name.to_string());
    let mut i = start + 1;

    while i < lines.len() {
        let line = lines[i];

        if line.starts_with("model ") {
            break; // Start of new model
        }

        if line.starts_with("graph ") {
            let (graph, new_i) = parse_single_graph(lines, i)?;
            model.add_part(graph);
            i = new_i;
        } else {
            i += 1; // Skip unknown lines
        }
    }

    Ok((model, i))
}

// Helper function to parse a single graph
fn parse_single_graph(lines: &[&str], start: usize) -> Result<(modality_lang::Part, usize), String> {
    if start >= lines.len() {
        return Err("Unexpected end of file".to_string());
    }

    let graph_line = lines[start];
    let graph_name = graph_line
        .strip_prefix("graph ")
        .and_then(|s| s.strip_suffix(':'))
        .ok_or_else(|| format!("Invalid graph declaration: {}", graph_line))?;

    let mut part = modality_lang::Part::new(graph_name.to_string());
    let mut i = start + 1;

    while i < lines.len() {
        let line = lines[i];

        if line.starts_with("model ") || line.starts_with("graph ") {
            break; // Start of new model or graph
        }

        if line.contains("-->") {
            let transition = parse_single_transition(line)?;
            part.add_transition(transition);
        }

        i += 1;
    }

    Ok((part, i))
}

// Helper function to parse a single transition
fn parse_single_transition(line: &str) -> Result<modality_lang::Transition, String> {
    let parts: Vec<&str> = line.split("-->").collect();
    if parts.len() != 2 {
        return Err(format!("Invalid transition format: {}", line));
    }

    let from = parts[0].trim();
    let to_and_props = parts[1].trim();

    if to_and_props.contains(':') {
        let colon_parts: Vec<&str> = to_and_props.split(':').collect();
        if colon_parts.len() != 2 {
            return Err(format!("Invalid transition format (invalid colon): {}", line));
        }

        let to = colon_parts[0].trim();
        let props_str = colon_parts[1].trim();

        let mut transition = modality_lang::Transition::new(from.to_string(), to.to_string());

        if !props_str.is_empty() {
            let properties = parse_single_properties(props_str)?;
            for prop in properties {
                transition.add_property(prop);
            }
        }

        Ok(transition)
    } else {
        let to = to_and_props;
        Ok(modality_lang::Transition::new(from.to_string(), to.to_string()))
    }
}

// Helper function to parse properties
fn parse_single_properties(props_str: &str) -> Result<Vec<modality_lang::Property>, String> {
    let mut properties = Vec::new();

    for prop_str in props_str.split_whitespace() {
        let prop = parse_single_property(prop_str)?;
        properties.push(prop);
    }

    Ok(properties)
}

// Helper function to parse a single property
fn parse_single_property(prop_str: &str) -> Result<modality_lang::Property, String> {
    if prop_str.is_empty() {
        return Err("Empty property".to_string());
    }

    let sign = match prop_str.chars().next() {
        Some('+') => modality_lang::PropertySign::Plus,
        Some('-') => modality_lang::PropertySign::Minus,
        _ => return Err(format!("Invalid property sign in: {}", prop_str)),
    };

    let name = prop_str[1..].to_string();
    if name.is_empty() {
        return Err(format!("Property name is empty: {}", prop_str));
    }

    Ok(modality_lang::Property::new(sign, name))
} 