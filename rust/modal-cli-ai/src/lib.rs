//! AI provider configuration and rule suggestion for Modal CLI.

pub mod complete;
pub mod config;
pub mod cursor_agent;
pub mod providers;
pub mod set;
pub mod show;
pub mod unset;

use anyhow::Result;
use clap::Subcommand;

/// Paths agents should read instead of searching `rust/`.
pub const LANGUAGE_SKILL_HELP: &str = "\
Rule formulas: docs/language/formula-cookbook.md
Witness models: docs/language/model-cookbook.md
Cursor skill: packages/modality-skill/SKILL.md";

#[derive(Debug, Subcommand)]
#[command(after_help = LANGUAGE_SKILL_HELP)]
pub enum Commands {
    /// Configure the AI provider used by modal ai suggest-rule
    Set(set::Opts),
    /// Show the configured AI provider
    Show(show::Opts),
    /// Remove the configured AI provider
    Unset(unset::Opts),
}

pub async fn run(command: &Commands) -> Result<()> {
    match command {
        Commands::Set(opts) => set::run(opts),
        Commands::Show(opts) => show::run(opts),
        Commands::Unset(opts) => unset::run(opts),
    }
}

pub use complete::{suggest_rule, suggest_rule_mode, suggest_rule_with};
pub use config::{unconfigured_error, AiConfig, Provider};
pub use cursor_agent::SuggestPrintMode;

#[cfg(test)]
mod tests;
