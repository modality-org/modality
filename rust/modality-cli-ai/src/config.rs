use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Openai,
    Anthropic,
    Grok,
    Bedrock,
    Ollama,
    #[serde(rename = "cursor-agent")]
    #[value(name = "cursor-agent")]
    CursorAgent,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Openai => "openai",
            Provider::Anthropic => "anthropic",
            Provider::Grok => "grok",
            Provider::Bedrock => "bedrock",
            Provider::Ollama => "ollama",
            Provider::CursorAgent => "cursor-agent",
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            Provider::Openai => "gpt-5.6-luna",
            Provider::Anthropic => "claude-sonnet-4-20250514",
            Provider::Grok => "grok-3",
            Provider::Bedrock => "anthropic.claude-sonnet-4-20250514-v1:0",
            Provider::Ollama => "llama3.2",
            Provider::CursorAgent => "auto",
        }
    }

    pub fn default_base_url(self) -> Option<&'static str> {
        match self {
            Provider::Openai => Some("https://api.openai.com"),
            Provider::Anthropic => Some("https://api.anthropic.com"),
            Provider::Grok => Some("https://api.x.ai"),
            Provider::Bedrock => None,
            Provider::Ollama => Some("http://127.0.0.1:11434"),
            Provider::CursorAgent => None,
        }
    }

    pub fn env_api_key(self) -> Option<&'static str> {
        match self {
            Provider::Openai => Some("OPENAI_API_KEY"),
            Provider::Anthropic => Some("ANTHROPIC_API_KEY"),
            Provider::Grok => Some("XAI_API_KEY"),
            Provider::CursorAgent => Some("CURSOR_API_KEY"),
            Provider::Bedrock | Provider::Ollama => None,
        }
    }

    pub fn requires_api_key(self) -> bool {
        !matches!(
            self,
            Provider::Bedrock | Provider::Ollama | Provider::CursorAgent
        )
    }
}

pub const PROVIDER_CHOICE_HINT: &str = "openai|anthropic|grok|bedrock|ollama|cursor-agent";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiConfig {
    pub provider: Option<Provider>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

impl AiConfig {
    pub fn provider(&self) -> Result<Provider> {
        self.provider.ok_or_else(unconfigured_error)
    }

    pub fn model(&self) -> Result<String> {
        let provider = self.provider()?;
        Ok(self
            .model
            .clone()
            .unwrap_or_else(|| provider.default_model().to_string()))
    }

    pub fn base_url(&self) -> Result<Option<String>> {
        let provider = self.provider()?;
        Ok(self
            .base_url
            .clone()
            .or_else(|| provider.default_base_url().map(|s| s.to_string())))
    }

    pub fn region(&self) -> String {
        self.region
            .clone()
            .or_else(|| std::env::var("AWS_REGION").ok())
            .or_else(|| std::env::var("AWS_DEFAULT_REGION").ok())
            .unwrap_or_else(|| "us-east-1".to_string())
    }

    pub fn resolve_api_key(&self, explicit: Option<&str>) -> Result<Option<String>> {
        if let Some(key) = explicit.filter(|k| !k.is_empty()) {
            return Ok(Some(key.to_string()));
        }
        if let Ok(key) = std::env::var("MODAL_AI_API_KEY") {
            if !key.is_empty() {
                return Ok(Some(key));
            }
        }
        let provider = self.provider()?;
        if let Some(var) = provider.env_api_key() {
            if let Ok(key) = std::env::var(var) {
                if !key.is_empty() {
                    return Ok(Some(key));
                }
            }
        }
        if let Some(key) = self.api_key.as_deref().filter(|k| !k.is_empty()) {
            return Ok(Some(key.to_string()));
        }
        Ok(None)
    }

    pub fn require_api_key(&self, explicit: Option<&str>) -> Result<String> {
        match self.resolve_api_key(explicit)? {
            Some(key) => Ok(key),
            None => {
                let provider = self.provider()?;
                let hint = provider
                    .env_api_key()
                    .map(|var| format!(" Set {var} or pass --api-key."))
                    .unwrap_or_default();
                bail!(
                    "No API key configured for provider '{}'.{hint}",
                    provider.as_str()
                );
            }
        }
    }
}

pub fn unconfigured_error() -> anyhow::Error {
    anyhow::anyhow!(
        "No AI provider configured. Run `modal ai set --provider {PROVIDER_CHOICE_HINT}`."
    )
}

pub fn config_path() -> Result<PathBuf> {
    modal_common::passfile::default_ai_config_path()
}

pub fn load() -> Result<Option<AiConfig>> {
    load_from(&config_path()?)
}

pub fn load_required() -> Result<AiConfig> {
    load()?.ok_or_else(unconfigured_error)
}

pub fn load_from(path: &Path) -> Result<Option<AiConfig>> {
    if !path.exists() {
        return Ok(None);
    }
    let text =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let config: AiConfig = serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(Some(config))
}

pub fn save(config: &AiConfig) -> Result<PathBuf> {
    save_to(&config_path()?, config)
}

pub fn save_to(path: &Path, config: &AiConfig) -> Result<PathBuf> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(config)?;
    fs::write(path, json).with_context(|| format!("Failed to write {}", path.display()))?;
    set_owner_only_permissions(path);
    Ok(path.to_path_buf())
}

pub fn unset() -> Result<bool> {
    unset_at(&config_path()?)
}

pub fn unset_at(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(path).with_context(|| format!("Failed to remove {}", path.display()))?;
    Ok(true)
}

fn set_owner_only_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
}

pub fn redact_key(key: &str) -> String {
    if key.len() <= 8 {
        return "****".to_string();
    }
    format!("{}…{}", &key[..4], &key[key.len() - 4..])
}
