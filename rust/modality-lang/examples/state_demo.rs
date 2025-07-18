use modality_lang::{parse_content_lalrpop, generate_mermaid_diagram_with_state, generate_mermaid_diagram};

fn main() -> Result<(), String> {
    println!("=== Non-Deterministic State Mermaid Demo ===\n");
    
    // Model with non-deterministic state (multiple possible states)
    let content = r#"
model NonDeterministicModel:
  graph g1:
    n1 --> n2: +blue -red
    n2 --> n3: +blue -green
    n3 --> n1: -blue +red
    n1 --> n3: +yellow
  graph g2:
    a --> b: +init
    b --> c: +complete
    c --> a: +reset
    a --> c: +skip
  state:
    g1: n1 n2
    g2: a b
"#;

    println!("📝 Model with Non-Deterministic State:");
    println!("{}", content);
    println!("{}", "=".repeat(60));
    
    // Parse the model
    let model = parse_content_lalrpop(content)?;
    
    println!("\n✅ Parsed Model:");
    println!("Name: {}", model.name);
    println!("Number of graphs: {}", model.graphs.len());
    
    if let Some(state) = &model.state {
        println!("\n🎯 Current State Information:");
        for graph_state in state {
            println!("  Graph '{}': {:?}", graph_state.graph_name, graph_state.current_nodes);
        }
    }
    
    println!("\n{}", "=".repeat(60));
    
    // Generate basic Mermaid diagram
    println!("\n📊 Basic Mermaid Diagram:");
    let basic_diagram = generate_mermaid_diagram(&model);
    println!("```mermaid");
    println!("{}", basic_diagram);
    println!("```");
    
    println!("\n{}", "=".repeat(60));
    
    // Generate state-aware Mermaid diagram
    println!("\n🎯 State-Aware Mermaid Diagram (Non-Deterministic):");
    let state_diagram = generate_mermaid_diagram_with_state(&model);
    println!("```mermaid");
    println!("{}", state_diagram);
    println!("```");
    
    println!("\n💡 Key Features:");
    println!("• Multiple current states per graph (non-deterministic)");
    println!("• g1: n1 and n2 are both highlighted as current states");
    println!("• g2: a and b are both highlighted as current states");
    println!("• Light blue background indicates current possible states");
    println!("• The system can be in any of the highlighted states");
    
    println!("\n=== Demo Complete ===");
    
    Ok(())
} 