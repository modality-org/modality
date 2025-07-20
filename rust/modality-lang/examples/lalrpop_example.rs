use modality_lang::{parse_all_models_lalrpop, PropertySign};

fn main() -> Result<(), String> {
    let file_path = "examples/models/SimpleExamples.modality";
    
    println!("=== LALRPOP Parser Example ===\n");
    
    // Parse all models using LALRPOP
    match parse_all_models_lalrpop(file_path) {
        Ok(models) => {
            println!("✓ Successfully parsed {} models:", models.len());
            
            for (i, model) in models.iter().enumerate() {
                println!("\n--- Model {}: {} ---", i + 1, model.name);
                println!("Number of parts: {}", model.parts.len());
                
                for (part_idx, part) in model.parts.iter().enumerate() {
                    println!("  Part {}: {} ({} transitions)", 
                             part_idx + 1, part.name, part.transitions.len());
                    
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
            }
        }
        Err(e) => println!("✗ Error parsing models: {}", e),
    }
    
    println!("\n=== LALRPOP Parser Example Complete ===");
    
    Ok(())
} 