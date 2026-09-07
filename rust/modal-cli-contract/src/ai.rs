use anyhow::Result;
use clap::{Parser, Subcommand};

/// First-contract authorized formula. Placeholder until suggest-rule calls a model.
pub const FIRST_CONTRACT_FORMULA: &str =
    "[] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)";

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
}

pub async fn run(command: &Commands) -> Result<()> {
    match command {
        Commands::SuggestRule(opts) => suggest_rule(opts),
    }
}

fn suggest_rule(_opts: &SuggestRuleOpts) -> Result<()> {
    println!("{FIRST_CONTRACT_FORMULA}");
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
        assert_eq!(
            FIRST_CONTRACT_FORMULA,
            "[] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)"
        );
    }
}
