use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config::{AiConfig, Provider};
use crate::providers;

/// Checked-in formula cookbook. Used when `docs/language/formula-cookbook.md` is not on disk.
pub const EMBEDDED_FORMULA_COOKBOOK: &str =
    include_str!("../../../docs/language/formula-cookbook.md");

const COOKBOOK_FILE: &str = "formula-cookbook.md";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestPrintMode {
    Print,
    Interactive,
}

#[derive(Debug, Clone)]
pub(crate) struct CursorAgentInvocation {
    pub bin: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub capture: bool,
}

pub async fn suggest(
    config: &AiConfig,
    requirement: &str,
    api_key: Option<&str>,
    contract_dir: Option<&Path>,
    identity_paths: &[String],
    print_mode: SuggestPrintMode,
) -> Result<String> {
    let cookbook = resolve_cookbook(contract_dir);
    let invocation = invocation(
        config,
        &build_prompt(requirement, identity_paths, cookbook.is_some()),
        api_key,
        contract_dir,
        print_mode,
    )?;
    let bin = invocation.bin.clone();
    let args = invocation.args.clone();
    let cwd = invocation.cwd.clone();
    let capture = invocation.capture;

    let output = tokio::task::spawn_blocking(move || run_command(&bin, &args, &cwd, capture))
        .await
        .context("cursor-agent task failed")??;

    if capture {
        providers::extract_formula(&output)
    } else {
        Ok(String::new())
    }
}

pub(crate) fn invocation(
    config: &AiConfig,
    prompt: &str,
    api_key: Option<&str>,
    contract_dir: Option<&Path>,
    print_mode: SuggestPrintMode,
) -> Result<CursorAgentInvocation> {
    if config.provider()? != Provider::CursorAgent {
        bail!("cursor-agent invocation requires provider cursor-agent");
    }
    let cwd = match contract_dir {
        Some(dir) => dir
            .canonicalize()
            .with_context(|| format!("Failed to resolve contract directory {}", dir.display()))?,
        None => std::env::current_dir()?,
    };
    let capture = wants_print(print_mode);
    let mut args = Vec::new();
    if capture {
        args.push("-p".to_string());
        args.push("--output-format".to_string());
        args.push("text".to_string());
        args.push("--trust".to_string());
    }
    args.push("--workspace".to_string());
    args.push(cwd.to_string_lossy().into_owned());
    for dir in extra_workspace_dirs(contract_dir) {
        args.push("--add-dir".to_string());
        args.push(dir.to_string_lossy().into_owned());
    }
    args.push("--mode".to_string());
    args.push("ask".to_string());
    let model = config.model()?;
    if model != "auto" {
        args.push("--model".to_string());
        args.push(model);
    }
    if let Some(key) = config.resolve_api_key(api_key)? {
        args.push("--api-key".to_string());
        args.push(key);
    }
    args.push("--".to_string());
    args.push(prompt.to_string());
    Ok(CursorAgentInvocation {
        bin: resolve_bin()?,
        args,
        cwd,
        capture,
    })
}

pub fn build_prompt(
    requirement: &str,
    identity_paths: &[String],
    cookbook_on_disk: bool,
) -> String {
    let mut out = String::from(
        "You are in a Modality contract directory. Read state/, rules/, and model/ as needed.\n\n",
    );
    out.push_str(
        "Read formula-cookbook.md first (model-cookbook.md for a witness; SKILL.md for contract ops). Do not search rust/, experiments/, or node_modules.\n\n",
    );
    if !cookbook_on_disk {
        out.push_str("Formula cookbook:\n\n");
        out.push_str(EMBEDDED_FORMULA_COOKBOOK);
        out.push_str("\n\n");
    }
    out.push_str("Requirement:\n");
    out.push_str(requirement.trim());
    if !identity_paths.is_empty() {
        out.push_str("\n\nKnown identity paths in this contract:\n");
        for path in identity_paths {
            out.push_str("- ");
            out.push_str(path);
            out.push('\n');
        }
    }
    out.push_str("\nWhen you have the answer, put the formula alone on the last line.\n");
    out
}

fn extra_workspace_dirs(contract_dir: Option<&Path>) -> Vec<PathBuf> {
    let Some(cookbook) = resolve_cookbook(contract_dir) else {
        return Vec::new();
    };
    let Some(language_dir) = cookbook.parent() else {
        return Vec::new();
    };
    let mut dirs = vec![language_dir.to_path_buf()];
    if let Some(skill) = language_dir
        .parent()
        .and_then(|docs| docs.parent())
        .map(|root| root.join("packages").join("modality-skill"))
    {
        if skill.join("SKILL.md").is_file() {
            dirs.push(skill);
        }
    }
    dirs
}

pub fn resolve_cookbook(contract_dir: Option<&Path>) -> Option<PathBuf> {
    if let Ok(docs) = std::env::var("MODALITY_DOCS") {
        let trimmed = docs.trim();
        if trimmed.is_empty() {
            return None;
        }
        return cookbook_from_docs_hint(&PathBuf::from(trimmed));
    }
    if let Some(found) = contract_dir
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok())
        .and_then(|start| walk_up_for_cookbook(&start))
    {
        return Some(found);
    }
    walk_up_for_cookbook(Path::new(env!("CARGO_MANIFEST_DIR")))
}

fn cookbook_from_docs_hint(hint: &Path) -> Option<PathBuf> {
    if hint.is_file() {
        let is_cookbook = hint
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == COOKBOOK_FILE);
        return is_cookbook.then(|| hint.to_path_buf());
    }
    [
        hint.join(COOKBOOK_FILE),
        hint.join("language").join(COOKBOOK_FILE),
        hint.join("docs").join("language").join(COOKBOOK_FILE),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

fn walk_up_for_cookbook(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent()?.to_path_buf()
    };
    loop {
        let candidate = dir.join("docs").join("language").join(COOKBOOK_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

pub fn resolve_bin() -> Result<PathBuf> {
    if let Ok(explicit) = std::env::var("MODAL_AI_CURSOR_AGENT") {
        if explicit.trim().is_empty() {
            bail!("MODAL_AI_CURSOR_AGENT is set but empty.");
        }
        return Ok(PathBuf::from(explicit));
    }
    for name in ["agent", "cursor-agent"] {
        if let Some(path) = find_in_path(name) {
            return Ok(path);
        }
    }
    Err(anyhow!(
        "Cursor Agent CLI not found. Install it from https://cursor.com/docs/cli, run `agent login`, then `modal ai set --provider cursor-agent`. To point at a binary, set MODAL_AI_CURSOR_AGENT."
    ))
}

fn wants_print(mode: SuggestPrintMode) -> bool {
    mode == SuggestPrintMode::Print
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|meta| meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn run_command(bin: &Path, args: &[String], cwd: &Path, capture: bool) -> Result<String> {
    let mut cmd = Command::new(bin);
    cmd.args(args).current_dir(cwd);
    if capture {
        let output = cmd
            .output()
            .with_context(|| format!("Failed to run {}", bin.display()))?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if !output.status.success() {
            let detail = if stderr.trim().is_empty() {
                stdout
            } else {
                stderr
            };
            bail!(
                "cursor-agent exited with {}: {detail}",
                output.status.code().unwrap_or(-1)
            );
        }
        Ok(stdout)
    } else {
        let status = cmd
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .with_context(|| format!("Failed to run {}", bin.display()))?;
        if !status.success() {
            bail!("cursor-agent exited with {}", status.code().unwrap_or(-1));
        }
        Ok(String::new())
    }
}
