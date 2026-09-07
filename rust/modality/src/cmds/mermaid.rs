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
    let model = load_named_model(&opts.input, opts.model.as_deref())?;
    println!("{}", modality_lang::generate_mermaid_diagram(&model));
    Ok(())
}

pub(crate) fn load_named_model(
    input: &str,
    model_name: Option<&str>,
) -> Result<modality_lang::Model> {
    let content = std::fs::read_to_string(input)?;
    let models = modality_lang::parse_all_models_content_lalrpop(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse models: {e}"))?;

    if let Some(model_name) = model_name {
        models
            .into_iter()
            .find(|m| m.name == *model_name)
            .ok_or_else(|| anyhow::anyhow!("Model '{model_name}' not found"))
    } else {
        models
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No models found in file"))
    }
}
