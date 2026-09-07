use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use crate::config::{self, AiConfig, Provider};

#[derive(Debug, Parser)]
#[command(about = "Configure the AI provider used by modal ai suggest-rule")]
pub struct Opts {
    /// Provider name
    #[clap(long, value_enum)]
    pub provider: Provider,

    /// Model id (provider default if omitted)
    #[clap(long)]
    pub model: Option<String>,

    /// API base URL (openai, anthropic, grok, ollama)
    #[clap(long)]
    pub base_url: Option<String>,

    /// AWS region (bedrock)
    #[clap(long)]
    pub region: Option<String>,

    /// API key (openai, anthropic, grok)
    #[clap(long)]
    pub api_key: Option<String>,

    /// Persist the API key in ~/.modality/ai.json (mode 0600)
    #[clap(long)]
    pub save_key: bool,
}

pub fn run(opts: &Opts) -> Result<()> {
    if opts.save_key && (opts.provider == Provider::Bedrock || opts.provider == Provider::Ollama) {
        anyhow::bail!(
            "--save-key is only supported for openai, anthropic, and grok. Bedrock uses the AWS credential chain; Ollama does not need an API key."
        );
    }
    if opts.save_key && opts.api_key.as_deref().unwrap_or("").is_empty() {
        anyhow::bail!("--save-key requires --api-key.");
    }

    let mut config = config::load()?.unwrap_or_default();
    config.provider = Some(opts.provider);
    config.model = opts.model.clone();
    config.base_url = opts.base_url.clone();
    config.region = opts.region.clone();
    if opts.save_key {
        config.api_key = opts.api_key.clone();
    } else {
        config.api_key = None;
    }

    let path = config::save(&config)?;
    print_set_summary(&config, &path, opts.save_key);
    if opts.api_key.is_some() && !opts.save_key {
        println!("   API key: not saved (pass --save-key to persist)");
    }
    Ok(())
}

fn print_set_summary(config: &AiConfig, path: &PathBuf, saved_key: bool) {
    let provider = config.provider.map(|p| p.as_str()).unwrap_or("unset");
    println!("✅ AI provider saved to {}", path.display());
    println!("   Provider: {provider}");
    if let Ok(model) = config.model() {
        println!("   Model: {model}");
    }
    if let Ok(Some(base)) = config.base_url() {
        println!("   Base URL: {base}");
    }
    if config.provider == Some(Provider::Bedrock) {
        println!("   Region: {}", config.region());
    }
    if saved_key {
        println!("   API key: saved");
    }
}
