use modality_lang::{
    generate_mermaid_diagram, generate_mermaid_diagram_with_styling, generate_mermaid_diagrams,
    parse_all_models_lalrpop,
};

fn main() -> Result<(), String> {
    let file_path = "test.modality";

    println!("=== Mermaid Diagram Generation Example ===\n");

    // Parse all models
    let models = parse_all_models_lalrpop(file_path)?;

    println!("✓ Successfully parsed {} models\n", models.len());

    // Generate diagrams for each model
    for model in models.iter() {
        println!("Model: {}", model.name);
        println!("Number of parts: {}", model.parts.len());

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
