use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Suggest a Modality rule from a plain-language prompt
    #[command(name = "suggest-rule")]
    SuggestRule(SuggestRuleOpts),
}

#[derive(Debug, Parser)]
pub struct SuggestRuleOpts {
    /// Plain-language description of the rule
    prompt: String,

    /// API key override (otherwise env or saved config)
    #[clap(long)]
    api_key: Option<String>,

    /// Contract directory used to include known identity paths
    #[clap(long)]
    dir: Option<PathBuf>,
}

pub async fn run(command: &Commands) -> Result<()> {
    match command {
        Commands::SuggestRule(opts) => suggest_rule(opts).await,
    }
}

async fn suggest_rule(opts: &SuggestRuleOpts) -> Result<()> {
    let dir = opts
        .dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let formula =
        modal_cli_ai::suggest_rule(&opts.prompt, opts.api_key.as_deref(), Some(dir.as_path()))
            .await?;
    println!("{formula}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_first_contract_prompt() {
        let opts = SuggestRuleOpts::parse_from([
            "suggest-rule",
            "after this commit either alice or bob must sign",
        ]);
        assert_eq!(
            opts.prompt,
            "after this commit either alice or bob must sign"
        );
    }
}
