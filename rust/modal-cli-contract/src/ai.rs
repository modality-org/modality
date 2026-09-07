use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
#[command(after_help = modal_cli_ai::LANGUAGE_SKILL_HELP)]
pub enum Commands {
    /// Suggest a Modality rule from a plain-language prompt
    #[command(name = "suggest-rule")]
    SuggestRule(SuggestRuleOpts),
}

#[derive(Debug, Parser)]
#[command(
    about = "Suggest a Modality rule from a plain-language prompt",
    long_about = "Suggest a Modality rule formula from a plain-language prompt for `modal add-rule`.\n\n\
Encodings follow docs/language/formula-cookbook.md. Witness models follow docs/language/model-cookbook.md.\n\
The Cursor skill packages/modality-skill/SKILL.md points at the same files."
)]
pub struct SuggestRuleOpts {
    /// Plain-language description of the rule
    prompt: String,

    /// API key override (otherwise env or saved config)
    #[clap(long)]
    api_key: Option<String>,

    /// Contract directory used to include known identity paths
    #[clap(long)]
    dir: Option<PathBuf>,

    /// Non-interactive cursor-agent print mode (the default)
    #[clap(long, conflicts_with = "interactive")]
    print: bool,

    /// Interactive cursor-agent session in the contract directory
    #[clap(long, conflicts_with = "print")]
    interactive: bool,
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
    let print_mode = if opts.interactive {
        modal_cli_ai::SuggestPrintMode::Interactive
    } else {
        modal_cli_ai::SuggestPrintMode::Print
    };
    let formula = modal_cli_ai::suggest_rule_mode(
        &opts.prompt,
        opts.api_key.as_deref(),
        Some(dir.as_path()),
        print_mode,
    )
    .await?;
    if !formula.is_empty() {
        println!("{formula}");
    }
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
        assert!(!opts.print);
        assert!(!opts.interactive);
    }

    #[test]
    fn parses_print_and_interactive_flags() {
        let print = SuggestRuleOpts::parse_from(["suggest-rule", "--print", "must sign"]);
        assert!(print.print);
        assert!(!print.interactive);

        let interactive =
            SuggestRuleOpts::parse_from(["suggest-rule", "--interactive", "must sign"]);
        assert!(interactive.interactive);
        assert!(!interactive.print);
    }

    #[test]
    fn default_is_print_unless_interactive() {
        let opts = SuggestRuleOpts::parse_from(["suggest-rule", "must sign"]);
        assert!(!opts.interactive);
        assert!(!opts.print);
    }

    #[test]
    fn suggest_rule_help_mentions_language_skill_files() {
        use clap::CommandFactory;
        let help = SuggestRuleOpts::command().render_long_help().to_string();
        assert!(help.contains("docs/language/formula-cookbook.md"));
        assert!(help.contains("docs/language/model-cookbook.md"));
        assert!(help.contains("packages/modality-skill/SKILL.md"));
    }
}
