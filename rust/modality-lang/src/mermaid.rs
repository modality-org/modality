use crate::ast::{Model, Graph, Transition, Property, PropertySign};

/// Generate a Mermaid state diagram from a Modality model
pub fn generate_mermaid_diagram(model: &Model) -> String {
    let mut diagram = String::new();
    diagram.push_str("stateDiagram-v2\n");
    
    for (_graph_idx, graph) in model.graphs.iter().enumerate() {
        if model.graphs.len() > 1 {
            diagram.push_str(&format!("    state {} {{\n", graph.name));
        }
        
        // Add all nodes first with graph prefix if multiple graphs
        let mut nodes = std::collections::HashSet::new();
        for transition in &graph.transitions {
            nodes.insert(&transition.from);
            nodes.insert(&transition.to);
        }
        
        for node in nodes {
            if model.graphs.len() > 1 {
                diagram.push_str(&format!("        {}.{} : {}\n", graph.name, node, node));
            } else {
                diagram.push_str(&format!("        {}\n", node));
            }
        }
        
        // Add all transitions within this graph only
        for transition in &graph.transitions {
            let edge_label = if transition.properties.is_empty() {
                String::new()
            } else {
                let props: Vec<String> = transition.properties
                    .iter()
                    .map(|prop| {
                        let sign = match prop.sign {
                            PropertySign::Plus => "+",
                            PropertySign::Minus => "-",
                        };
                        format!("{}{}", sign, prop.name)
                    })
                    .collect();
                format!(" : {}", props.join(" "))
            };
            
            if model.graphs.len() > 1 {
                diagram.push_str(&format!("        {}.{} --> {}.{}{}\n", 
                    graph.name, transition.from, graph.name, transition.to, edge_label));
            } else {
                diagram.push_str(&format!("        {} --> {}{}\n", 
                    transition.from, transition.to, edge_label));
            }
        }
        
        if model.graphs.len() > 1 {
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
    
    for (_graph_idx, graph) in model.graphs.iter().enumerate() {
        if model.graphs.len() > 1 {
            diagram.push_str(&format!("    state {} {{\n", graph.name));
        }
        
        // Add all nodes first with graph prefix if multiple graphs
        let mut nodes = std::collections::HashSet::new();
        for transition in &graph.transitions {
            nodes.insert(&transition.from);
            nodes.insert(&transition.to);
        }
        
        for node in nodes {
            if model.graphs.len() > 1 {
                diagram.push_str(&format!("        {}.{} : {}\n", graph.name, node, node));
            } else {
                diagram.push_str(&format!("        {}\n", node));
            }
        }
        
        // Add all transitions within this graph only
        for transition in &graph.transitions {
            let edge_label = if transition.properties.is_empty() {
                String::new()
            } else {
                let props: Vec<String> = transition.properties
                    .iter()
                    .map(|prop| {
                        let sign = match prop.sign {
                            PropertySign::Plus => "+",
                            PropertySign::Minus => "-",
                        };
                        format!("{}{}", sign, prop.name)
                    })
                    .collect();
                format!(" : {}", props.join(" "))
            };
            
            if model.graphs.len() > 1 {
                diagram.push_str(&format!("        {}.{} --> {}.{}{}\n", 
                    graph.name, transition.from, graph.name, transition.to, edge_label));
            } else {
                diagram.push_str(&format!("        {} --> {}{}\n", 
                    transition.from, transition.to, edge_label));
            }
        }
        
        if model.graphs.len() > 1 {
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
    
    for (_graph_idx, graph) in model.graphs.iter().enumerate() {
        if model.graphs.len() > 1 {
            diagram.push_str(&format!("    state {} {{\n", graph.name));
        }
        
        // Add all nodes first with graph prefix if multiple graphs
        let mut nodes = std::collections::HashSet::new();
        for transition in &graph.transitions {
            nodes.insert(&transition.from);
            nodes.insert(&transition.to);
        }
        
        // Check if this graph has current state information
        let empty_vec = Vec::<String>::new();
        let current_nodes = if let Some(state) = &model.state {
            state.iter()
                .find(|s| s.graph_name == graph.name)
                .map(|s| &s.current_nodes)
                .unwrap_or(&empty_vec)
        } else {
            &empty_vec
        };
        
        for node in nodes {
            let node_name = if model.graphs.len() > 1 {
                format!("{}.{}", graph.name, node)
            } else {
                node.to_string()
            };
            
            if model.graphs.len() > 1 {
                diagram.push_str(&format!("        {}.{} : {}\n", graph.name, node, node));
            } else {
                diagram.push_str(&format!("        {}\n", node));
            }
        }
        
        // Add all transitions within this graph only
        for transition in &graph.transitions {
            let edge_label = if transition.properties.is_empty() {
                String::new()
            } else {
                let props: Vec<String> = transition.properties
                    .iter()
                    .map(|prop| {
                        let sign = match prop.sign {
                            PropertySign::Plus => "+",
                            PropertySign::Minus => "-",
                        };
                        format!("{}{}", sign, prop.name)
                    })
                    .collect();
                format!(" : {}", props.join(" "))
            };
            
            if model.graphs.len() > 1 {
                diagram.push_str(&format!("        {}.{} --> {}.{}{}\n", 
                    graph.name, transition.from, graph.name, transition.to, edge_label));
            } else {
                diagram.push_str(&format!("        {} --> {}{}\n", 
                    transition.from, transition.to, edge_label));
            }
        }
        
        if model.graphs.len() > 1 {
            diagram.push_str("    }\n");
        }
    }
    
    // Apply current state styling
    if let Some(state) = &model.state {
        for graph_state in state {
            for node in &graph_state.current_nodes {
                let node_name = if model.graphs.len() > 1 {
                    format!("{}.{}", graph_state.graph_name, node)
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
    use crate::ast::{Model, Graph, Transition, Property, PropertySign};

    #[test]
    fn test_generate_simple_diagram() {
        let mut model = Model::new("TestModel".to_string());
        let mut graph = Graph::new("g1".to_string());
        
        let transition = Transition::new("n1".to_string(), "n2".to_string());
        graph.add_transition(transition);
        
        model.add_graph(graph);
        
        let diagram = generate_mermaid_diagram(&model);
        
        assert!(diagram.contains("stateDiagram-v2"));
        assert!(diagram.contains("n1"));
        assert!(diagram.contains("n2"));
        assert!(diagram.contains("n1 --> n2"));
    }

    #[test]
    fn test_generate_diagram_with_properties() {
        let mut model = Model::new("TestModel".to_string());
        let mut graph = Graph::new("g1".to_string());
        
        let mut transition = Transition::new("n1".to_string(), "n2".to_string());
        transition.add_property(Property::new(PropertySign::Plus, "blue".to_string()));
        transition.add_property(Property::new(PropertySign::Minus, "red".to_string()));
        graph.add_transition(transition);
        
        model.add_graph(graph);
        
        let diagram = generate_mermaid_diagram(&model);
        
        assert!(diagram.contains("n1 --> n2 : +blue -red"));
    }

    #[test]
    fn test_generate_multiple_graphs_diagram() {
        let mut model = Model::new("TestModel".to_string());
        
        // First graph
        let mut graph1 = Graph::new("g1".to_string());
        let transition1 = Transition::new("n1".to_string(), "n2".to_string());
        graph1.add_transition(transition1);
        model.add_graph(graph1);
        
        // Second graph
        let mut graph2 = Graph::new("g2".to_string());
        let transition2 = Transition::new("a".to_string(), "b".to_string());
        graph2.add_transition(transition2);
        model.add_graph(graph2);
        
        let diagram = generate_mermaid_diagram(&model);
        
        assert!(diagram.contains("state g1 {"));
        assert!(diagram.contains("state g2 {"));
        assert!(diagram.contains("g1.n1 : n1"));
        assert!(diagram.contains("g1.n2 : n2"));
        assert!(diagram.contains("g2.a : a"));
        assert!(diagram.contains("g2.b : b"));
        assert!(diagram.contains("g1.n1 --> g1.n2"));
        assert!(diagram.contains("g2.a --> g2.b"));
        
        // Verify graphs are isolated - no transitions between graphs
        assert!(!diagram.contains("g1.n1 --> g2.a"));
        assert!(!diagram.contains("g2.a --> g1.n1"));
    }

    #[test]
    fn test_graph_isolation() {
        let mut model = Model::new("TestModel".to_string());
        
        // Graph 1 with nodes n1, n2
        let mut graph1 = Graph::new("g1".to_string());
        let transition1 = Transition::new("n1".to_string(), "n2".to_string());
        graph1.add_transition(transition1);
        model.add_graph(graph1);
        
        // Graph 2 with nodes a, b
        let mut graph2 = Graph::new("g2".to_string());
        let transition2 = Transition::new("a".to_string(), "b".to_string());
        graph2.add_transition(transition2);
        model.add_graph(graph2);
        
        let diagram = generate_mermaid_diagram(&model);
        
        // Verify each graph only contains its own nodes and transitions
        let lines: Vec<&str> = diagram.lines().collect();
        
        // Find the g1 state block
        let mut in_g1 = false;
        let mut g1_nodes = std::collections::HashSet::new();
        let mut g1_transitions = std::collections::HashSet::new();
        
        // Find the g2 state block
        let mut in_g2 = false;
        let mut g2_nodes = std::collections::HashSet::new();
        let mut g2_transitions = std::collections::HashSet::new();
        
        for line in lines {
            let trimmed = line.trim();
            if trimmed == "state g1 {" {
                in_g1 = true;
                in_g2 = false;
            } else if trimmed == "state g2 {" {
                in_g1 = false;
                in_g2 = true;
            } else if trimmed == "}" {
                in_g1 = false;
                in_g2 = false;
            } else if in_g1 && !trimmed.is_empty() && !trimmed.starts_with("state ") {
                if trimmed.contains("-->") {
                    g1_transitions.insert(trimmed.to_string());
                } else {
                    g1_nodes.insert(trimmed.to_string());
                }
            } else if in_g2 && !trimmed.is_empty() && !trimmed.starts_with("state ") {
                if trimmed.contains("-->") {
                    g2_transitions.insert(trimmed.to_string());
                } else {
                    g2_nodes.insert(trimmed.to_string());
                }
            }
        }
        
        // Verify g1 only contains g1.n1, g1.n2 and their transition
        assert!(g1_nodes.contains("g1.n1 : n1"));
        assert!(g1_nodes.contains("g1.n2 : n2"));
        assert!(!g1_nodes.contains("g2.a : a"));
        assert!(!g1_nodes.contains("g2.b : b"));
        assert!(g1_transitions.iter().any(|t| t.contains("g1.n1 --> g1.n2")));
        assert!(!g1_transitions.iter().any(|t| t.contains("g2.a") || t.contains("g2.b")));
        
        // Verify g2 only contains g2.a, g2.b and their transition
        assert!(g2_nodes.contains("g2.a : a"));
        assert!(g2_nodes.contains("g2.b : b"));
        assert!(!g2_nodes.contains("g1.n1 : n1"));
        assert!(!g2_nodes.contains("g1.n2 : n2"));
        assert!(g2_transitions.iter().any(|t| t.contains("g2.a --> g2.b")));
        assert!(!g2_transitions.iter().any(|t| t.contains("g1.n1") || t.contains("g1.n2")));
    }

    #[test]
    fn test_single_graph_no_prefix() {
        let mut model = Model::new("TestModel".to_string());
        let mut graph = Graph::new("g1".to_string());
        
        let transition = Transition::new("n1".to_string(), "n2".to_string());
        graph.add_transition(transition);
        
        model.add_graph(graph);
        
        let diagram = generate_mermaid_diagram(&model);
        
        // Single graph should not use prefixes
        assert!(diagram.contains("n1"));
        assert!(diagram.contains("n2"));
        assert!(diagram.contains("n1 --> n2"));
        assert!(!diagram.contains("g1.n1"));
        assert!(!diagram.contains("g1.n2"));
    }

    #[test]
    fn test_generate_diagram_with_state() {
        use crate::ast::GraphState;
        
        let mut model = Model::new("TestModel".to_string());
        let mut graph = Graph::new("g1".to_string());
        
        let transition = Transition::new("n1".to_string(), "n2".to_string());
        graph.add_transition(transition);
        model.add_graph(graph);
        
        // Add state information
        let state = vec![
            GraphState::new("g1".to_string(), vec!["n1".to_string(), "n2".to_string()])
        ];
        model.set_state(state);
        
        let diagram = generate_mermaid_diagram_with_state(&model);
        
        // Should contain current state styling
        assert!(diagram.contains("classDef current fill:#e3f2fd,stroke:#1976d2,stroke-width:3px"));
        assert!(diagram.contains("class n1 current"));
        assert!(diagram.contains("class n2 current"));
    }

    #[test]
    fn test_generate_diagram_with_multiple_graphs_and_state() {
        use crate::ast::GraphState;
        
        let mut model = Model::new("TestModel".to_string());
        
        // First graph
        let mut graph1 = Graph::new("g1".to_string());
        let transition1 = Transition::new("n1".to_string(), "n2".to_string());
        graph1.add_transition(transition1);
        model.add_graph(graph1);
        
        // Second graph
        let mut graph2 = Graph::new("g2".to_string());
        let transition2 = Transition::new("a".to_string(), "b".to_string());
        graph2.add_transition(transition2);
        model.add_graph(graph2);
        
        // Add state information
        let state = vec![
            GraphState::new("g1".to_string(), vec!["n1".to_string()]),
            GraphState::new("g2".to_string(), vec!["a".to_string(), "b".to_string()])
        ];
        model.set_state(state);
        
        let diagram = generate_mermaid_diagram_with_state(&model);
        
        // Should contain current state styling with prefixed names
        assert!(diagram.contains("classDef current fill:#e3f2fd,stroke:#1976d2,stroke-width:3px"));
        assert!(diagram.contains("class g1.n1 current"));
        assert!(diagram.contains("class g2.a current"));
        assert!(diagram.contains("class g2.b current"));
    }
} 