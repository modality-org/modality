use anyhow::Result;
use clap::Parser;

use crate::config::{self, AiConfig, Provider};

#[derive(Debug, Parser)]
#[command(about = "Show the configured AI provider")]
pub struct Opts {}

pub fn run(_opts: &Opts) -> Result<()> {
    let Some(config) = config::load()? else {
        anyhow::bail!("{}", config::unconfigured_error());
    };
    print!("{}", format_show(&config)?);
    Ok(())
}

pub fn format_show(config: &AiConfig) -> Result<String> {
    let provider = config.provider()?;
    let mut out = String::new();
    out.push_str(&format!("Provider: {}\n", provider.as_str()));
    out.push_str(&format!("Model: {}\n", config.model()?));
    if let Some(base) = config.base_url()? {
        out.push_str(&format!("Base URL: {base}\n"));
    }
    if provider == Provider::Bedrock {
        out.push_str(&format!("Region: {}\n", config.region()));
    }
    match config.resolve_api_key(None)? {
        Some(key) => out.push_str(&format!("API key: {}\n", config::redact_key(&key))),
        None if !provider.requires_api_key() => match provider {
            Provider::CursorAgent => {
                out.push_str("API key: (not required; `agent login` or CURSOR_API_KEY)\n");
            }
            _ => out.push_str("API key: (not required)\n"),
        },
        None => out.push_str("API key: (not set; use env or --api-key)\n"),
    }
    Ok(out)
}
