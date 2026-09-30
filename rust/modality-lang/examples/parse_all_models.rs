//! Parse every model in a file and print its parts and transitions.
//!
//! Run: `cargo run -p modality-lang --example parse_all_models [file.modality]`.
//! Without a file it reads `examples/models/SimpleExamples.modality`.

use modality_lang::{parse_all_models_lalrpop, PropertySign, Transition};

fn print_transitions(transitions: &[Transition]) {
    for transition in transitions {
        print!("    {} --> {}:", transition.from, transition.to);
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

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/models/SimpleExamples.modality").to_string()
    });
    let models = parse_all_models_lalrpop(&path)?;

    for (n, model) in models.iter().enumerate() {
        println!("\n=== Model {} ===", n + 1);
        println!("Model name: {}", model.name);
        if let Some(initial) = &model.initial {
            println!("Initial node: {}", initial);
        }
        if !model.transitions.is_empty() {
            println!("  Transitions outside a part: {}", model.transitions.len());
            print_transitions(&model.transitions);
        }
        println!("Number of parts: {}", model.parts.len());
        for (part_idx, part) in model.parts.iter().enumerate() {
            println!("  Part {}: {}", part_idx + 1, part.name);
            println!("    Transitions: {}", part.transitions.len());
            print_transitions(&part.transitions);
        }
    }

    println!("\nTotal models found: {}", models.len());
    Ok(())
}
