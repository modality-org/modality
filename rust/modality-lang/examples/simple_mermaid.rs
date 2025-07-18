use modality_lang::{parse_content_lalrpop, generate_mermaid_diagram};

fn main() -> Result<(), String> {
    // Define a simple model
    let model_content = r#"
model SimpleModel:
  graph main:
    start --> processing: +init
    processing --> success: +complete
    processing --> error: +fail
    success --> end
    error --> end
"#;

    // Parse the model
    let model = parse_content_lalrpop(model_content)?;
    
    println!("Model: {}", model.name);
    println!("Graphs: {}", model.graphs.len());
    
    // Generate and display the Mermaid diagram
    let diagram = generate_mermaid_diagram(&model);
    
    println!("\nGenerated Mermaid Diagram:");
    println!("```mermaid");
    println!("{}", diagram);
    println!("```");
    
    println!("\nYou can copy this diagram into any Mermaid-compatible viewer!");
    
    Ok(())
} 