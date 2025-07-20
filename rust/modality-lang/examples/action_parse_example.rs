use modality_lang::{parse_all_actions_content_lalrpop, parse_action_call_lalrpop, PropertySign};

fn main() -> Result<(), String> {
    let content = r#"
action ActionHello: +hello
action ActionHelloFriend: +hello +friend
action ActionComplex: +blue -red +green
"#;
    
    println!("=== Action Parsing Example ===\n");
    
    // Parse all actions
    match parse_all_actions_content_lalrpop(content) {
        Ok(actions) => {
            println!("✓ Successfully parsed {} actions:", actions.len());
            
            for (i, action) in actions.iter().enumerate() {
                println!("\n--- Action {}: {} ---", i + 1, action.name);
                println!("Number of properties: {}", action.properties.len());
                
                for (prop_idx, prop) in action.properties.iter().enumerate() {
                    let sign = match prop.sign {
                        PropertySign::Plus => "+",
                        PropertySign::Minus => "-",
                    };
                    println!("  Property {}: {}{}", prop_idx + 1, sign, prop.name);
                }
            }
        }
        Err(e) => println!("✗ Error parsing actions: {}", e),
    }
    
    // Test action call parsing
    println!("\n=== Action Call Parsing ===");
    
    let action_call_content = r#"action("+hello")"#;
    match parse_action_call_lalrpop(action_call_content) {
        Ok(action_call) => {
            println!("✓ Successfully parsed action call:");
            println!("  Argument: {}", action_call.argument);
        }
        Err(e) => println!("✗ Error parsing action call: {}", e),
    }
    
    println!("\n=== Action Parsing Example Complete ===");
    
    Ok(())
} 