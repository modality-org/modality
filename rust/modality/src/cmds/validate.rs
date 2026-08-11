use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// Validate a contract model (predicates/method labels, no raw propositions)
#[derive(Parser, Debug)]
pub struct Opts {
    /// Path to the .modality file
    pub file: PathBuf,

    /// Show detailed validation info
    #[arg(short, long)]
    pub verbose: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    println!("🔍 Validating contract: {}\n", opts.file.display());

    // Parse the model
    let model = modality_lang::parse_file_lalrpop(&opts.file)
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    if opts.verbose {
        println!("📋 Model: {}", model.name);
        println!("   Parts: {}", model.parts.len());
        let transition_count: usize = model.parts.iter().map(|p| p.transitions.len()).sum();
        println!("   Transitions: {}", transition_count);
        println!();
    }

    // Validate no raw propositions
    match modality_lang::validation::validate_no_raw_propositions(&model) {
        Ok(()) => {
            println!("✅ Contract is valid!");
            println!(
                "   All properties are predicates or commit method labels (verifier-observed)."
            );
            Ok(())
        }
        Err(errors) => {
            println!("❌ Contract validation failed!\n");
            println!(
                "   Contracts require predicates or commit method labels, not raw propositions."
            );
            println!("   Predicates and method labels are verifier-observed; propositions are just claims.\n");

            for error in &errors {
                println!("   ⚠️  {}", error);

                // Add helpful suggestion
                if let modality_lang::validation::ValidationError::RawProposition {
                    property_name,
                    ..
                } = error
                {
                    let suggestion = modality_lang::validation::suggest_predicate(property_name);
                    println!("      💡 Try: +{}", suggestion);
                }
                println!();
            }

            println!("📚 Available predicates:");
            for pred in modality_lang::validation::KNOWN_PREDICATES {
                println!("   - {}(...)", pred);
            }

            Err(anyhow::anyhow!(
                "Validation failed with {} error(s)",
                errors.len()
            ))
        }
    }
}
