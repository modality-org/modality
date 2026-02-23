use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modal_common::contract_store::ContractStore;

#[derive(Debug, Parser)]
#[command(about = "Add a rule to the contract")]
pub struct Opts {
    /// Rule formula (e.g. "always (+signed_by(/users/alice.id))")
    #[clap(index = 1)]
    formula: String,

    /// Rule name (auto-generated if not provided)
    #[clap(long)]
    name: Option<String>,

    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let dir = opts.dir.clone().unwrap_or_else(|| std::env::current_dir().unwrap());
    let store = ContractStore::open(&dir)?;

    let rule_name = opts.name.clone().unwrap_or_else(|| {
        format!("rule_{}", chrono::Utc::now().timestamp())
    });

    let rule_content = format!(
        "export default rule {{\n  formula {{\n    {}\n  }}\n}}\n",
        opts.formula
    );

    // Write to rules directory
    store.init_rules_dir()?;
    let rule_path = format!("/rules/{}.modality", rule_name);
    store.write_rule(&rule_path, &serde_json::Value::String(rule_content.clone()))?;

    println!("✅ Rule '{}' added to {}", rule_name, rule_path);
    println!();
    println!("{}", rule_content);
    println!("Run 'modal commit --all' to commit this rule.");

    Ok(())
}
