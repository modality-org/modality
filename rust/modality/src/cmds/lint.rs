use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// Lint Modality governance formulas for common semantic mistakes
#[derive(Parser, Debug)]
pub struct Opts {
    /// Path to a .modality file containing `formula` or rule `formula` blocks (and optionally a witness model)
    pub file: PathBuf,

    /// Witness model file for witness-node-leak checks (defaults to models in `file`)
    #[arg(long)]
    pub model: Option<PathBuf>,

    /// Treat warnings as errors (non-zero exit)
    #[arg(long, default_value_t = true)]
    pub deny_warnings: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let content = std::fs::read_to_string(&opts.file)?;
    let witness_model = if let Some(model_path) = &opts.model {
        Some(
            modality_lang::parse_file_lalrpop(model_path)
                .map_err(|e| anyhow::anyhow!("Failed to parse witness model: {e}"))?,
        )
    } else {
        modality_lang::parse_all_models_content_lalrpop(&content)
            .ok()
            .and_then(|models| models.into_iter().next())
    };

    let lint_opts = modality_lang::FormulaLintOptions { witness_model };

    let results = modality_lang::lint_formulas_in_content(&content, &lint_opts)
        .map_err(|e| anyhow::anyhow!("Failed to lint formulas: {e}"))?;

    if results.is_empty() {
        println!("ℹ️  No `formula` blocks found in {}", opts.file.display());
        return Ok(());
    }

    let mut warning_count = 0usize;
    for (name, diags) in &results {
        for diag in diags {
            warning_count += 1;
            let icon = match diag.severity {
                modality_lang::LintSeverity::Warning => "⚠️ ",
                modality_lang::LintSeverity::Hint => "💡",
            };
            println!(
                "{icon}  {} ({}) in formula `{name}`",
                diag.message,
                diag.code.as_str()
            );
            if let Some(suggestion) = &diag.suggestion {
                println!("      → {suggestion}");
            }
        }
    }

    if warning_count == 0 {
        println!(
            "✅ {} formula(s) lint-clean in {}",
            results.len(),
            opts.file.display()
        );
        return Ok(());
    }

    println!(
        "\n❌ {warning_count} lint finding(s) in {} formula(s)",
        results.len()
    );

    if opts.deny_warnings {
        Err(anyhow::anyhow!(
            "Formula lint failed with {warning_count} finding(s)"
        ))
    } else {
        Ok(())
    }
}
