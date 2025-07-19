use anyhow::Result;
use clap::Parser;

/// Check a formula against a model
#[derive(Parser, Debug)]
pub struct Opts {
    /// Path to the .modality file
    pub input: String,
    
    /// Name of the model to check (optional, defaults to first model)
    #[arg(short, long)]
    pub model: Option<String>,
    
    /// Name of the formula to check (optional, if not provided will use --formula-text)
    #[arg(short, long)]
    pub formula: Option<String>,
    
    /// Formula text to check (optional, if not provided will use --formula)
    #[arg(long)]
    pub formula_text: Option<String>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Read the file
    let content = std::fs::read_to_string(&opts.input)?;
    
    // Parse all models from the file
    let models = modality_lang::parse_all_models_content_lalrpop(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse models: {}", e))?;
    
    // Find the specific model or use the first one
    let model = if let Some(model_name) = &opts.model {
        models.into_iter()
            .find(|m| m.name == *model_name)
            .ok_or_else(|| anyhow::anyhow!("Model '{}' not found", model_name))?
    } else {
        models.into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No models found in file"))?
    };
    
    // Get the formula to check
    let formula = if let Some(formula_name) = &opts.formula {
        // Parse all formulas from the file and find the named one
        let formulas = modality_lang::parse_all_formulas_content_lalrpop(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse formulas: {}", e))?;
        
        formulas.into_iter()
            .find(|f| f.name == *formula_name)
            .ok_or_else(|| anyhow::anyhow!("Formula '{}' not found", formula_name))?
    } else if let Some(formula_text) = &opts.formula_text {
        // Parse the formula text directly
        let formula_content = format!("formula TempFormula: {}", formula_text);
        let formulas = modality_lang::parse_all_formulas_content_lalrpop(&formula_content)
            .map_err(|e| anyhow::anyhow!("Failed to parse formula text: {}", e))?;
        
        formulas.into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Failed to parse formula text"))?
    } else {
        return Err(anyhow::anyhow!("Must specify either --formula or --formula-text"));
    };
    
    // Create model checker and check the formula
    let checker = modality_lang::ModelChecker::new(model);
    let result = checker.check_formula(&formula);
    let result_any_state = checker.check_formula_any_state(&formula);
    
    // Output results
    println!("🔍 Checking formula: {}", formula.name);
    println!("📋 Formula: {:?}", formula.expression);
    println!("");
    
    if result.is_satisfied {
        println!("✅ Formula is satisfied (per-graph requirement)");
    } else {
        println!("❌ Formula is not satisfied (per-graph requirement)");
    }
    
    if result_any_state.is_satisfied {
        println!("✅ Formula is satisfied (any state)");
    } else {
        println!("❌ Formula is not satisfied (any state)");
    }
    
    println!("");
    println!("📍 Satisfying states ({}):", result.satisfying_states.len());
    for state in &result.satisfying_states {
        println!("   - {}.{}", state.graph_name, state.node_name);
    }
    
    Ok(())
} 