use anyhow::Result;
use std::path::Path;

use crate::config::{self, AiConfig};
use crate::providers::{self, JsonPoster, ReqwestPoster};

pub async fn suggest_rule(
    prompt: &str,
    api_key: Option<&str>,
    contract_dir: Option<&Path>,
) -> Result<String> {
    let config = config::load_required()?;
    let poster = ReqwestPoster::new()?;
    suggest_rule_with(&config, prompt, api_key, contract_dir, &poster).await
}

pub async fn suggest_rule_with(
    config: &AiConfig,
    prompt: &str,
    api_key: Option<&str>,
    contract_dir: Option<&Path>,
    poster: &dyn JsonPoster,
) -> Result<String> {
    let user_prompt = build_user_prompt(prompt, contract_dir);
    let raw = providers::complete(config, &user_prompt, api_key, poster).await?;
    providers::extract_formula(&raw)
}

fn build_user_prompt(prompt: &str, contract_dir: Option<&Path>) -> String {
    let mut out = providers::SUGGEST_RULE_FEW_SHOT.to_string();
    out.push_str("\nRequirement:\n");
    out.push_str(prompt.trim());
    if let Some(dir) = contract_dir {
        let ids = collect_id_paths(dir);
        if !ids.is_empty() {
            out.push_str("\n\nKnown identity paths in this contract:\n");
            for path in ids {
                out.push_str("- ");
                out.push_str(&path);
                out.push('\n');
            }
        }
    }
    out.push_str("\n\nFormula:\n");
    out
}

fn collect_id_paths(contract_dir: &Path) -> Vec<String> {
    let state_dir = contract_dir.join("state");
    let mut paths = Vec::new();
    collect_id_paths_from(&state_dir, &state_dir, &mut paths);
    paths.sort();
    paths
}

fn collect_id_paths_from(state_dir: &Path, dir: &Path, paths: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_id_paths_from(state_dir, &path, paths);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("id") {
            continue;
        }
        if let Ok(relative) = path.strip_prefix(state_dir) {
            paths.push(format!(
                "/{}",
                relative.to_string_lossy().replace('\\', "/")
            ));
        }
    }
}
