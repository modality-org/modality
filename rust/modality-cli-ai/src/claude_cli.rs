use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};

use crate::config::{AiConfig, Provider};
use crate::cursor_agent::{
    build_prompt, extra_workspace_dirs, find_in_path, resolve_cookbook, run_command,
    SuggestPrintMode,
};
use crate::providers;

/// Tools the CLI may use: it reads the contract and the cookbooks, and changes nothing.
const READ_ONLY_TOOLS: &str = "Read,Glob,Grep";

/// Same prompt and parsing as cursor-agent, run through the Claude Code CLI
/// (`claude -p`), which signs in with an Anthropic account or API key.
pub async fn suggest(
    config: &AiConfig,
    requirement: &str,
    contract_dir: Option<&Path>,
    identity_paths: &[String],
    print_mode: SuggestPrintMode,
) -> Result<String> {
    let cookbook = resolve_cookbook(contract_dir);
    let prompt = build_prompt(requirement, identity_paths, cookbook.is_some());
    let capture = print_mode == SuggestPrintMode::Print;
    let (bin, args, cwd) = invocation(config, &prompt, contract_dir, capture)?;
    let output = tokio::task::spawn_blocking(move || run_command(&bin, &args, &cwd, capture))
        .await
        .context("claude task failed")??;
    if capture {
        providers::extract_formula(&output)
    } else {
        Ok(String::new())
    }
}

/// Run the Claude Code CLI once in print mode with `prompt` in `workspace`,
/// and return what it printed.
pub async fn run_prompt(config: &AiConfig, prompt: &str, workspace: &Path) -> Result<String> {
    let (bin, args, cwd) = invocation(config, prompt, Some(workspace), true)?;
    tokio::task::spawn_blocking(move || run_command(&bin, &args, &cwd, true))
        .await
        .context("claude task failed")?
}

pub(crate) fn invocation(
    config: &AiConfig,
    prompt: &str,
    contract_dir: Option<&Path>,
    capture: bool,
) -> Result<(PathBuf, Vec<String>, PathBuf)> {
    if config.provider()? != Provider::ClaudeCli {
        bail!("claude-cli invocation requires provider claude-cli");
    }
    let cwd = match contract_dir {
        Some(dir) => dir
            .canonicalize()
            .with_context(|| format!("Failed to resolve contract directory {}", dir.display()))?,
        None => std::env::current_dir()?,
    };
    let mut args = Vec::new();
    if capture {
        args.extend(
            ["-p", "--output-format", "text", "--no-session-persistence"].map(String::from),
        );
    }
    args.extend(["--model".to_string(), config.model()?]);
    args.extend(
        [
            "--tools",
            READ_ONLY_TOOLS,
            "--allowedTools",
            READ_ONLY_TOOLS,
        ]
        .map(String::from),
    );
    // No MCP servers or user settings: the answer depends on the prompt and the model only.
    args.extend(["--strict-mcp-config", "--setting-sources", ""].map(String::from));
    for dir in extra_workspace_dirs(contract_dir) {
        args.push("--add-dir".to_string());
        args.push(dir.to_string_lossy().into_owned());
    }
    args.push("--".to_string());
    args.push(prompt.to_string());
    Ok((resolve_bin()?, args, cwd))
}

pub fn resolve_bin() -> Result<PathBuf> {
    if let Ok(explicit) = std::env::var("MODAL_AI_CLAUDE_CLI") {
        if explicit.trim().is_empty() {
            bail!("MODAL_AI_CLAUDE_CLI is set but empty.");
        }
        return Ok(PathBuf::from(explicit));
    }
    find_in_path("claude").ok_or_else(|| {
        anyhow!(
            "Claude Code CLI not found. Install it (https://docs.claude.com/en/docs/claude-code/setup), run `claude` once to sign in, then `modal ai set --provider claude-cli`. To point at a binary, set MODAL_AI_CLAUDE_CLI."
        )
    })
}
