use modality_lang::{parse_file, parse_file_lalrpop, PropertySign};

fn main() -> Result<(), String> {
    let file_path = "examples/models/SimpleExamples.modality";
    
    println!("=== Comparing Parsers ===\n");
    
    // Test hand-written parser
    println!("1. Testing hand-written parser:");
    match parse_file(file_path) {
        Ok(model) => {
            println!("   ✓ Successfully parsed model: {}", model.name);
            println!("   ✓ Number of graphs: {}", model.graphs.len());
            for (i, graph) in model.graphs.iter().enumerate() {
                println!("   ✓ Graph {}: {} ({} transitions)", i + 1, graph.name, graph.transitions.len());
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }
    
    println!();
    
    // Test LALRPOP parser
    println!("2. Testing LALRPOP parser:");
    match parse_file_lalrpop(file_path) {
        Ok(model) => {
            println!("   ✓ Successfully parsed model: {}", model.name);
            println!("   ✓ Number of graphs: {}", model.graphs.len());
            for (i, graph) in model.graphs.iter().enumerate() {
                println!("   ✓ Graph {}: {} ({} transitions)", i + 1, graph.name, graph.transitions.len());
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }
    
    println!();
    
    // Test with a simple example
    println!("3. Testing with simple example:");
    let simple_content = r#"
model TestModel:
  graph g1:
    n1 --> n2: +blue
    n2 --> n3: -red
"#;
    
    println!("   Input:");
    println!("   {}", simple_content.trim());
    println!();
    
    match modality_lang::parse_content_lalrpop(simple_content) {
        Ok(model) => {
            println!("   ✓ LALRPOP parser result:");
            println!("   ✓ Model: {}", model.name);
            println!("   ✓ Graphs: {}", model.graphs.len());
            
            for graph in &model.graphs {
                println!("   ✓ Graph: {} ({} transitions)", graph.name, graph.transitions.len());
                for transition in &graph.transitions {
                    print!("   ✓   {} --> {}:", transition.from, transition.to);
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
        Err(e) => println!("   ✗ LALRPOP parser error: {}", e),
    }
    
    println!();
    println!("=== Parser Comparison Complete ===");
    
    Ok(())
} 