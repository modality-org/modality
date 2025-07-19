use anyhow::Result;
use clap::Parser;

/// Generate a Mermaid diagram from a Modality file
#[derive(Parser, Debug)]
pub struct Opts {
    /// Path to the .modality file
    pub input: String,
    
    /// Name of the model to generate diagram for (optional, defaults to first model)
    #[arg(short, long)]
    pub model: Option<String>,
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
    
    // Generate the mermaid diagram
    let mermaid = modality_lang::generate_mermaid_diagram(&model);
    // Output to stdout
    println!("{}", mermaid);
    Ok(())
} 