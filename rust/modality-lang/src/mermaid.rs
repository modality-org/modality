use crate::ast::{Model, Part, Transition, Property, PropertySign, PartState};

/// Generate a Mermaid state diagram from a Modality model
pub fn generate_mermaid_diagram(model: &Model) -> String {
    let mut diagram = String::new();
    diagram.push_str("stateDiagram-v2\n");
    
    for (_part_idx, part) in model.parts.iter().enumerate() {
        if model.parts.len() > 1 {
            diagram.push_str(&format!("    state {} {{\n", part.name));
        }
        
        // Add all nodes first with part prefix if multiple parts
        let mut nodes = std::collections::HashSet::new();
        for transition in &part.transitions {
            nodes.insert(&transition.from);
            nodes.insert(&transition.to);
        }
        
        for node in nodes {
            if model.parts.len() > 1 {
                diagram.push_str(&format!("        {}.{} : {}\n", part.name, node, node));
            } else {
                diagram.push_str(&format!("        {}\n", node));
            }
        }
        
        // Add all transitions within this part only
        for transition in &part.transitions {
            let edge_label = if transition.properties.is_empty() {
                String::new()
            } else {
                let props: Vec<String> = transition.properties.iter()
                    .map(|p| format!("{}{}", if p.sign == PropertySign::Plus { "+" } else { "-" }, p.name))
                    .collect();
                format!(" : {}", props.join(" "))
            };
            
            if model.parts.len() > 1 {
                diagram.push_str(&format!("        {}.{} --> {}.{}{}\n", 
                    part.name, transition.from, part.name, transition.to, edge_label));
            } else {
                diagram.push_str(&format!("        {} --> {}{}\n", 
                    transition.from, transition.to, edge_label));
            }
        }
        
        if model.parts.len() > 1 {
            diagram.push_str("    }\n");
        }
    }
    
    diagram
}

/// Generate a Mermaid state diagram from multiple models
pub fn generate_mermaid_diagrams(models: &[Model]) -> String {
    let mut diagrams = String::new();
    
    for (i, model) in models.iter().enumerate() {
        diagrams.push_str(&format!("%% Model {}: {}\n", i + 1, model.name));
        diagrams.push_str(&generate_mermaid_diagram(model));
        if i < models.len() - 1 {
            diagrams.push_str("\n");
        }
    }
    
    diagrams
}

/// Generate a Mermaid state diagram with custom styling
pub fn generate_mermaid_diagram_with_styling(model: &Model) -> String {
    let mut diagram = String::new();
    diagram.push_str("stateDiagram-v2\n");
    
    // Add styling
    diagram.push_str("    classDef default fill:#f9f9f9,stroke:#333,stroke-width:2px\n");
    diagram.push_str("    classDef start fill:#d4edda,stroke:#155724,stroke-width:2px\n");
    diagram.push_str("    classDef end fill:#f8d7da,stroke:#721c24,stroke-width:2px\n");
    diagram.push_str("    classDef property fill:#fff3cd,stroke:#856404,stroke-width:2px\n");
    diagram.push_str("    classDef current fill:#e3f2fd,stroke:#1976d2,stroke-width:3px\n");
    
    for (_part_idx, part) in model.parts.iter().enumerate() {
        if model.parts.len() > 1 {
            diagram.push_str(&format!("    state {} {{\n", part.name));
        }
        
        // Add all nodes first with part prefix if multiple parts
        let mut nodes = std::collections::HashSet::new();
        for transition in &part.transitions {
            nodes.insert(&transition.from);
            nodes.insert(&transition.to);
        }
        
        for node in nodes {
            if model.parts.len() > 1 {
                diagram.push_str(&format!("        {}.{} : {}\n", part.name, node, node));
            } else {
                diagram.push_str(&format!("        {}\n", node));
            }
        }
        
        // Add all transitions within this part only
        for transition in &part.transitions {
            let edge_label = if transition.properties.is_empty() {
                String::new()
            } else {
                let props: Vec<String> = transition.properties.iter()
                    .map(|p| format!("{}{}", if p.sign == PropertySign::Plus { "+" } else { "-" }, p.name))
                    .collect();
                format!(" : {}", props.join(" "))
            };
            
            if model.parts.len() > 1 {
                diagram.push_str(&format!("        {}.{} --> {}.{}{}\n", 
                    part.name, transition.from, part.name, transition.to, edge_label));
            } else {
                diagram.push_str(&format!("        {} --> {}{}\n", 
                    transition.from, transition.to, edge_label));
            }
        }
        
        if model.parts.len() > 1 {
            diagram.push_str("    }\n");
        }
    }
    
    // Apply default styling to all nodes
    diagram.push_str("    class * default\n");
    
    diagram
}

/// Generate a Mermaid state diagram with current state highlighting
pub fn generate_mermaid_diagram_with_state(model: &Model) -> String {
    let mut diagram = String::new();
    diagram.push_str("stateDiagram-v2\n");
    
    // Add styling for current states
    diagram.push_str("    classDef current fill:#e3f2fd,stroke:#1976d2,stroke-width:3px\n");
    
    for (_part_idx, part) in model.parts.iter().enumerate() {
        if model.parts.len() > 1 {
            diagram.push_str(&format!("    state {} {{\n", part.name));
        }
        
        // Add all nodes first with part prefix if multiple parts
        let mut nodes = std::collections::HashSet::new();
        for transition in &part.transitions {
            nodes.insert(&transition.from);
            nodes.insert(&transition.to);
        }
        
        // Check if this part has current state information
        let empty_vec = Vec::<String>::new();
        let current_nodes = if let Some(state) = &model.state {
            state.iter()
                .find(|s| s.part_name == part.name)
                .map(|s| &s.current_nodes)
                .unwrap_or(&empty_vec)
        } else {
            &empty_vec
        };
        
        for node in nodes {
            let node_name = if model.parts.len() > 1 {
                format!("{}.{}", part.name, node)
            } else {
                node.to_string()
            };
            
            if model.parts.len() > 1 {
                diagram.push_str(&format!("        {}.{} : {}\n", part.name, node, node));
            } else {
                diagram.push_str(&format!("        {}\n", node));
            }
        }
        
        // Add all transitions within this part only
        for transition in &part.transitions {
            let edge_label = if transition.properties.is_empty() {
                String::new()
            } else {
                let props: Vec<String> = transition.properties.iter()
                    .map(|p| format!("{}{}", if p.sign == PropertySign::Plus { "+" } else { "-" }, p.name))
                    .collect();
                format!(" : {}", props.join(" "))
            };
            
            if model.parts.len() > 1 {
                diagram.push_str(&format!("        {}.{} --> {}.{}{}\n", 
                    part.name, transition.from, part.name, transition.to, edge_label));
            } else {
                diagram.push_str(&format!("        {} --> {}{}\n", 
                    transition.from, transition.to, edge_label));
            }
        }
        
        if model.parts.len() > 1 {
            diagram.push_str("    }\n");
        }
    }
    
    // Apply current state styling
    if let Some(state) = &model.state {
        for part_state in state {
            for node in &part_state.current_nodes {
                let node_name = if model.parts.len() > 1 {
                    format!("{}.{}", part_state.part_name, node)
                } else {
                    node.to_string()
                };
                diagram.push_str(&format!("    class {} current\n", node_name));
            }
        }
    }
    
    diagram
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Model, Part, Transition, Property, PropertySign, PartState};

    #[test]
    fn test_generate_simple_diagram() {
        let mut model = Model::new("TestModel".to_string());
        let mut part = Part::new("p1".to_string());
        
        let transition = Transition::new("n1".to_string(), "n2".to_string());
        part.add_transition(transition);
        
        model.add_part(part);
        
        let diagram = generate_mermaid_diagram(&model);
        
        assert!(diagram.contains("stateDiagram-v2"));
        assert!(diagram.contains("n1"));
        assert!(diagram.contains("n2"));
        assert!(diagram.contains("n1 --> n2"));
    }

    #[test]
    fn test_generate_diagram_with_properties() {
        let mut model = Model::new("TestModel".to_string());
        let mut part = Part::new("p1".to_string());
        
        let mut transition = Transition::new("n1".to_string(), "n2".to_string());
        transition.add_property(Property::new(PropertySign::Plus, "blue".to_string()));
        transition.add_property(Property::new(PropertySign::Minus, "red".to_string()));
        part.add_transition(transition);
        
        model.add_part(part);
        
        let diagram = generate_mermaid_diagram(&model);
        
        assert!(diagram.contains("n1 --> n2 : +blue -red"));
    }

    #[test]
    fn test_generate_multiple_parts_diagram() {
        let mut model = Model::new("TestModel".to_string());
        
        // First part
        let mut part1 = Part::new("p1".to_string());
        let transition1 = Transition::new("n1".to_string(), "n2".to_string());
        part1.add_transition(transition1);
        model.add_part(part1);
        
        // Second part
        let mut part2 = Part::new("p2".to_string());
        let transition2 = Transition::new("a".to_string(), "b".to_string());
        part2.add_transition(transition2);
        model.add_part(part2);
        
        let diagram = generate_mermaid_diagram(&model);
        
        assert!(diagram.contains("state p1 {"));
        assert!(diagram.contains("state p2 {"));
        assert!(diagram.contains("p1.n1 : n1"));
        assert!(diagram.contains("p1.n2 : n2"));
        assert!(diagram.contains("p2.a : a"));
        assert!(diagram.contains("p2.b : b"));
        assert!(diagram.contains("p1.n1 --> p1.n2"));
        assert!(diagram.contains("p2.a --> p2.b"));
        
        // Verify parts are isolated - no transitions between parts
        assert!(!diagram.contains("p1.n1 --> p2.a"));
        assert!(!diagram.contains("p2.a --> p1.n1"));
    }

    #[test]
    fn test_part_isolation() {
        let mut model = Model::new("TestModel".to_string());
        
        // Part 1 with nodes n1, n2
        let mut part1 = Part::new("p1".to_string());
        let transition1 = Transition::new("n1".to_string(), "n2".to_string());
        part1.add_transition(transition1);
        model.add_part(part1);
        
        // Part 2 with nodes a, b
        let mut part2 = Part::new("p2".to_string());
        let transition2 = Transition::new("a".to_string(), "b".to_string());
        part2.add_transition(transition2);
        model.add_part(part2);
        
        let diagram = generate_mermaid_diagram(&model);
        
        // Verify each part only contains its own nodes and transitions
        let lines: Vec<&str> = diagram.lines().collect();
        
        // Find the p1 state block
        let mut in_p1 = false;
        let mut p1_nodes = std::collections::HashSet::new();
        let mut p1_transitions = std::collections::HashSet::new();
        
        // Find the p2 state block
        let mut in_p2 = false;
        let mut p2_nodes = std::collections::HashSet::new();
        let mut p2_transitions = std::collections::HashSet::new();
        
        for line in lines {
            let trimmed = line.trim();
            if trimmed == "state p1 {" {
                in_p1 = true;
                in_p2 = false;
            } else if trimmed == "state p2 {" {
                in_p1 = false;
                in_p2 = true;
            } else if trimmed == "}" {
                in_p1 = false;
                in_p2 = false;
            } else if in_p1 && !trimmed.is_empty() && !trimmed.starts_with("state ") {
                if trimmed.contains("-->") {
                    p1_transitions.insert(trimmed.to_string());
                } else {
                    p1_nodes.insert(trimmed.to_string());
                }
            } else if in_p2 && !trimmed.is_empty() && !trimmed.starts_with("state ") {
                if trimmed.contains("-->") {
                    p2_transitions.insert(trimmed.to_string());
                } else {
                    p2_nodes.insert(trimmed.to_string());
                }
            }
        }
        
        // Verify p1 only contains p1.n1, p1.n2 and their transition
        assert!(p1_nodes.contains("p1.n1 : n1"));
        assert!(p1_nodes.contains("p1.n2 : n2"));
        assert!(!p1_nodes.contains("p2.a : a"));
        assert!(!p1_nodes.contains("p2.b : b"));
        assert!(p1_transitions.iter().any(|t| t.contains("p1.n1 --> p1.n2")));
        assert!(!p1_transitions.iter().any(|t| t.contains("p2.a") || t.contains("p2.b")));
        
        // Verify p2 only contains p2.a, p2.b and their transition
        assert!(p2_nodes.contains("p2.a : a"));
        assert!(p2_nodes.contains("p2.b : b"));
        assert!(!p2_nodes.contains("p1.n1 : n1"));
        assert!(!p2_nodes.contains("p1.n2 : n2"));
        assert!(p2_transitions.iter().any(|t| t.contains("p2.a --> p2.b")));
        assert!(!p2_transitions.iter().any(|t| t.contains("p1.n1") || t.contains("p1.n2")));
    }

    #[test]
    fn test_single_part_no_prefix() {
        let mut model = Model::new("TestModel".to_string());
        let mut part = Part::new("p1".to_string());
        
        let transition = Transition::new("n1".to_string(), "n2".to_string());
        part.add_transition(transition);
        
        model.add_part(part);
        
        let diagram = generate_mermaid_diagram(&model);
        
        // Single part should not use prefixes
        assert!(diagram.contains("n1"));
        assert!(diagram.contains("n2"));
        assert!(diagram.contains("n1 --> n2"));
        assert!(!diagram.contains("p1.n1"));
        assert!(!diagram.contains("p1.n2"));
    }

    #[test]
    fn test_generate_diagram_with_state() {
        use crate::ast::PartState;
        
        let mut model = Model::new("TestModel".to_string());
        let mut part = Part::new("p1".to_string());
        
        let transition = Transition::new("n1".to_string(), "n2".to_string());
        part.add_transition(transition);
        model.add_part(part);
        
        // Add state information
        let state = vec![
            PartState::new("p1".to_string(), vec!["n1".to_string()])
        ];
        model.set_state(state);
        
        let diagram = generate_mermaid_diagram_with_state(&model);
        
        // Should contain current state styling
        assert!(diagram.contains("classDef current fill:#e3f2fd,stroke:#1976d2,stroke-width:3px"));
        assert!(diagram.contains("class n1 current"));
        // n2 should not have current styling since it's not in the current state
        assert!(!diagram.contains("class n2 current"));
    }

    #[test]
    fn test_generate_diagram_with_multiple_parts_and_state() {
        use crate::ast::PartState;
        
        let mut model = Model::new("TestModel".to_string());
        
        // First part
        let mut part1 = Part::new("p1".to_string());
        let transition1 = Transition::new("n1".to_string(), "n2".to_string());
        part1.add_transition(transition1);
        model.add_part(part1);
        
        // Second part
        let mut part2 = Part::new("p2".to_string());
        let transition2 = Transition::new("a".to_string(), "b".to_string());
        part2.add_transition(transition2);
        model.add_part(part2);
        
        // Add state information
        let state = vec![
            PartState::new("p1".to_string(), vec!["n1".to_string()]),
            PartState::new("p2".to_string(), vec!["a".to_string()])
        ];
        model.set_state(state);
        
        let diagram = generate_mermaid_diagram_with_state(&model);
        
        // Should contain current state styling with prefixed names
        assert!(diagram.contains("classDef current fill:#e3f2fd,stroke:#1976d2,stroke-width:3px"));
        assert!(diagram.contains("class p1.n1 current"));
        assert!(diagram.contains("class p2.a current"));
        // p2.b should not have current styling since it's not in the current state
        assert!(!diagram.contains("class p2.b current"));
    }
} 