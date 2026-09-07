use anyhow::{bail, Result};
use clap::Parser;
use std::path::PathBuf;

use modal_common::contract_store::ContractStore;

#[derive(Debug, Parser)]
#[command(about = "Add a rule to the contract")]
pub struct Opts {
    /// Rule formula (e.g. "[] always([-signed_by(/parties/alice.id)] false)")
    #[clap(index = 1)]
    formula: String,

    /// Rule name written as rules/<name>.modality
    #[clap(long)]
    name: String,

    /// Anchor for the generated rule (default: $PARENT)
    #[clap(long, default_value = "$PARENT")]
    starting_at: String,

    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
}

pub fn format_rule(formula: &str, starting_at: &str) -> String {
    format!(
        "export default rule {{\n  starting_at {}\n  formula {{\n    {}\n  }}\n}}\n",
        starting_at, formula
    )
}

fn validate_rule_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Rule name must not be empty");
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        bail!("Rule name must be a single path segment, not {name:?}");
    }
    Ok(())
}

pub async fn run(opts: &Opts) -> Result<()> {
    validate_rule_name(&opts.name)?;

    let dir = opts
        .dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    let store = ContractStore::open(&dir)?;

    let rule_content = format_rule(&opts.formula, &opts.starting_at);

    store.init_rules_dir()?;
    let rule_path = format!("/rules/{}.modality", opts.name);
    let file_path = store.rules_dir().join(format!("{}.modality", opts.name));
    if file_path.exists() {
        bail!("Rule file already exists: {}", file_path.display());
    }
    store.write_rule(&rule_path, &serde_json::Value::String(rule_content.clone()))?;

    println!("✅ Rule '{}' added to {}", opts.name, rule_path);
    println!();
    println!("{}", rule_content);
    println!("Run 'modal c commit --all' to commit this rule.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use tempfile::TempDir;

    const FIRST_CONTRACT_FORMULA: &str =
        "[] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)";

    const FIRST_CONTRACT_RULE: &str = "export default rule {\n  starting_at $PARENT\n  formula {\n    [] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)\n  }\n}\n";

    #[test]
    fn formats_first_contract_rule() {
        assert_eq!(
            format_rule(FIRST_CONTRACT_FORMULA, "$PARENT"),
            FIRST_CONTRACT_RULE
        );
    }

    #[tokio::test]
    async fn writes_named_rule_and_refuses_overwrite() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let contract_dir = temp_dir.path().join("first-contract");
        let contract_dir_arg = contract_dir.to_string_lossy().to_string();

        let create_opts = crate::create::Opts::parse_from([
            "create",
            "--dir",
            contract_dir_arg.as_str(),
            "--output",
            "json",
        ]);
        crate::create::run(&create_opts).await?;

        let add_opts = Opts::parse_from([
            "add-rule",
            "--name",
            "authorized",
            "--dir",
            contract_dir_arg.as_str(),
            FIRST_CONTRACT_FORMULA,
        ]);
        run(&add_opts).await?;

        let rule_path = contract_dir.join("rules/authorized.modality");
        assert_eq!(std::fs::read_to_string(&rule_path)?, FIRST_CONTRACT_RULE);

        let again = run(&add_opts).await;
        assert!(again.is_err(), "add-rule should refuse to overwrite");
        assert!(
            again.unwrap_err().to_string().contains("already exists"),
            "overwrite error should mention the existing file"
        );
        assert_eq!(std::fs::read_to_string(&rule_path)?, FIRST_CONTRACT_RULE);

        Ok(())
    }
}
