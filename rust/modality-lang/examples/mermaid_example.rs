use modality_lang::{parse_all_models_lalrpop, generate_mermaid_diagram, generate_mermaid_diagrams, generate_mermaid_diagram_with_styling};

fn main() -> Result<(), String> {
    let file_path = "examples/models/SimpleExamples.modality";
    
    println!("=== Mermaid Diagram Generation Example ===\n");
    
    // Parse all models
    let models = parse_all_models_lalrpop(file_path)?;
    
    println!("✓ Successfully parsed {} models\n", models.len());
    
    // Generate diagrams for each model
    for (i, model) in models.iter().enumerate() {
        println!("--- Model {}: {} ---", i + 1, model.name);
        println!("Number of graphs: {}", model.graphs.len());
        
        // Generate basic Mermaid diagram
        let diagram = generate_mermaid_diagram(model);
        println!("\nBasic Mermaid Diagram:");
        println!("```mermaid");
        println!("{}", diagram);
        println!("```");
        
        // Generate styled diagram
        let styled_diagram = generate_mermaid_diagram_with_styling(model);
        println!("\nStyled Mermaid Diagram:");
        println!("```mermaid");
        println!("{}", styled_diagram);
        println!("```");
        
        println!("\n{}", "=".repeat(50));
        println!();
    }
    
    // Generate combined diagram for all models
    println!("--- Combined Diagram for All Models ---");
    let combined_diagram = generate_mermaid_diagrams(&models);
    println!("```mermaid");
    println!("{}", combined_diagram);
    println!("```");
    
    println!("\n=== Mermaid Diagram Generation Complete ===");
    
    Ok(())
} 