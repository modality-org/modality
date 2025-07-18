use modality_lang::{parse_file, PropertySign};

fn main() -> Result<(), String> {
    // Parse the example file
    let model = parse_file("examples/models/SimpleExamples.modality")?;
    
    println!("Parsed model: {}", model.name);
    println!("Number of graphs: {}", model.graphs.len());
    
    for (i, graph) in model.graphs.iter().enumerate() {
        println!("  Graph {}: {}", i + 1, graph.name);
        println!("    Transitions: {}", graph.transitions.len());
        
        for transition in &graph.transitions {
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
    
    Ok(())
} 