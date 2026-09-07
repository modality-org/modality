use std::sync::Mutex;

use anyhow::Result;
use clap::Parser;
use serde_json::{json, Value};
use tempfile::TempDir;

use crate::complete::suggest_rule_with;
use crate::config::{self, AiConfig, Provider};
use crate::providers::{self, JsonPoster};
use crate::set;
use crate::show;

const FORMULA: &str =
    "[] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)";
const SECRET_KEY: &str = "sk-test-secret-key-123456";

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    keys: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn capture(keys: &[&'static str]) -> Self {
        Self {
            keys: keys
                .iter()
                .map(|key| (*key, std::env::var(key).ok()))
                .collect(),
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.keys {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

struct MockPoster {
    response: Value,
    url: Mutex<Option<String>>,
    headers: Mutex<Vec<(String, String)>>,
    body: Mutex<Option<Value>>,
}

impl MockPoster {
    fn new(response: Value) -> Self {
        Self {
            response,
            url: Mutex::new(None),
            headers: Mutex::new(Vec::new()),
            body: Mutex::new(None),
        }
    }

    fn url(&self) -> String {
        self.url.lock().unwrap().clone().unwrap_or_default()
    }

    fn headers(&self) -> Vec<(String, String)> {
        self.headers.lock().unwrap().clone()
    }

    fn body(&self) -> Value {
        self.body.lock().unwrap().clone().unwrap_or(Value::Null)
    }
}

#[async_trait::async_trait]
impl JsonPoster for MockPoster {
    async fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &Value,
    ) -> Result<Value> {
        *self.url.lock().unwrap() = Some(url.to_string());
        *self.headers.lock().unwrap() = headers.to_vec();
        *self.body.lock().unwrap() = Some(body.clone());
        Ok(self.response.clone())
    }
}

fn openai_response(text: &str) -> Value {
    json!({"choices":[{"message":{"content": text}}]})
}

fn anthropic_response(text: &str) -> Value {
    json!({"content":[{"text": text}]})
}

fn ollama_response(text: &str) -> Value {
    json!({"message":{"content": text}})
}

fn bedrock_response(text: &str) -> Value {
    json!({"output":{"message":{"content":[{"text": text}]}}})
}

fn with_isolated_home<T>(f: impl FnOnce(&TempDir) -> T) -> T {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let _guard = EnvGuard::capture(&[
        "MODALITY_HOME",
        "MODAL_AI_API_KEY",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "XAI_API_KEY",
        "AWS_ACCESS_KEY_ID",
        "AWS_SECRET_ACCESS_KEY",
        "AWS_SESSION_TOKEN",
        "AWS_REGION",
        "AWS_DEFAULT_REGION",
        "CURSOR_API_KEY",
        "MODAL_AI_CURSOR_AGENT",
    ]);
    for key in [
        "MODAL_AI_API_KEY",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "XAI_API_KEY",
        "AWS_ACCESS_KEY_ID",
        "AWS_SECRET_ACCESS_KEY",
        "AWS_SESSION_TOKEN",
        "AWS_REGION",
        "AWS_DEFAULT_REGION",
        "CURSOR_API_KEY",
        "MODAL_AI_CURSOR_AGENT",
    ] {
        std::env::remove_var(key);
    }
    let home = TempDir::new().expect("tempdir");
    std::env::set_var("MODALITY_HOME", home.path());
    f(&home)
}

fn block_on<T>(fut: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
        .block_on(fut)
}

async fn suggest_with(
    provider: Provider,
    poster: &MockPoster,
    api_key: Option<&str>,
    contract_dir: Option<&std::path::Path>,
) -> Result<String> {
    let config = AiConfig {
        provider: Some(provider),
        ..Default::default()
    };
    suggest_rule_with(
        &config,
        "after this commit either alice or bob must sign",
        api_key,
        contract_dir,
        poster,
    )
    .await
}

#[test]
fn config_round_trip_saves_mode_and_show_redacts_key() {
    with_isolated_home(|home| {
        let config = AiConfig {
            provider: Some(Provider::Openai),
            model: Some("gpt-5.6-luna".to_string()),
            base_url: None,
            region: None,
            api_key: Some(SECRET_KEY.to_string()),
        };
        let path = config::save(&config).expect("save");
        assert!(path.starts_with(home.path()));
        assert!(path.ends_with("ai.json"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }

        let loaded = config::load_required().expect("load");
        assert_eq!(loaded.provider, Some(Provider::Openai));
        assert_eq!(loaded.api_key.as_deref(), Some(SECRET_KEY));

        let shown = show::format_show(&loaded).expect("show");
        assert!(shown.contains("Provider: openai"));
        assert!(shown.contains(&config::redact_key(SECRET_KEY)));
        assert!(!shown.contains(SECRET_KEY));

        assert!(config::unset().expect("unset"));
        assert!(config::load().expect("load missing").is_none());
    });
}

#[test]
fn unconfigured_error_points_at_modal_ai_set() {
    with_isolated_home(|_| {
        let err = config::load_required().expect_err("unconfigured");
        let message = format!("{err:#}");
        assert!(message.contains("modal ai set"));
        assert!(message.contains("openai|anthropic|grok|bedrock|ollama|cursor-agent"));
    });
}

#[test]
fn api_key_resolution_order() {
    with_isolated_home(|_| {
        let config = AiConfig {
            provider: Some(Provider::Openai),
            api_key: Some("saved-key-value-xxxx".to_string()),
            ..Default::default()
        };

        assert_eq!(
            config.resolve_api_key(Some("explicit-key")).unwrap(),
            Some("explicit-key".to_string())
        );

        std::env::set_var("MODAL_AI_API_KEY", "modal-env-key");
        assert_eq!(
            config.resolve_api_key(None).unwrap(),
            Some("modal-env-key".to_string())
        );

        std::env::remove_var("MODAL_AI_API_KEY");
        std::env::set_var("OPENAI_API_KEY", "openai-env-key");
        assert_eq!(
            config.resolve_api_key(None).unwrap(),
            Some("openai-env-key".to_string())
        );

        std::env::remove_var("OPENAI_API_KEY");
        assert_eq!(
            config.resolve_api_key(None).unwrap(),
            Some("saved-key-value-xxxx".to_string())
        );
    });
}

#[test]
fn suggest_rule_prompt_includes_first_contract_or_signers_example() {
    let prompt = providers::suggest_rule_system_prompt();
    assert!(prompt
        .contains("[] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)"));
    assert!(prompt.contains("Do not invent action names"));
    assert!(prompt.contains("[] φ` constrains successors of the current state"));
    assert!(
        !prompt.contains("prefixed with F1:"),
        "suggest-rule must not send the hub synthesis pattern table"
    );
}

#[test]
fn extract_formula_from_plain_and_wrapped_text() {
    assert_eq!(providers::extract_formula(FORMULA).unwrap(), FORMULA);
    let wrapped = format!("```\n{FORMULA}\n```");
    assert_eq!(providers::extract_formula(&wrapped).unwrap(), FORMULA);
    let err = providers::extract_formula("sorry, I cannot help with that").unwrap_err();
    assert!(format!("{err:#}").contains("sorry, I cannot help with that"));
}

#[test]
fn openai_mock_prints_extracted_formula() {
    with_isolated_home(|_| {
        let poster = MockPoster::new(openai_response(FORMULA));
        let formula = block_on(suggest_with(
            Provider::Openai,
            &poster,
            Some("sk-test"),
            None,
        ))
        .expect("suggest");
        assert_eq!(formula, FORMULA);
        assert!(poster.url().ends_with("/v1/chat/completions"));
        assert!(poster.url().contains("api.openai.com"));
        assert!(poster
            .headers()
            .iter()
            .any(|(name, value)| name == "Authorization" && value == "Bearer sk-test"));
    });
}

#[test]
fn anthropic_mock_prints_extracted_formula() {
    with_isolated_home(|_| {
        let poster = MockPoster::new(anthropic_response(FORMULA));
        let formula = block_on(suggest_with(
            Provider::Anthropic,
            &poster,
            Some("anthropic-key"),
            None,
        ))
        .expect("suggest");
        assert_eq!(formula, FORMULA);
        assert!(poster.url().ends_with("/v1/messages"));
        let headers = poster.headers();
        assert!(headers
            .iter()
            .any(|(name, value)| name == "x-api-key" && value == "anthropic-key"));
        assert!(headers
            .iter()
            .any(|(name, value)| name == "anthropic-version" && value == "2023-06-01"));
    });
}

#[test]
fn grok_mock_prints_extracted_formula() {
    with_isolated_home(|_| {
        let poster = MockPoster::new(openai_response(FORMULA));
        let formula = block_on(suggest_with(Provider::Grok, &poster, Some("xai-key"), None))
            .expect("suggest");
        assert_eq!(formula, FORMULA);
        assert!(poster.url().contains("api.x.ai"));
        assert!(poster.url().ends_with("/v1/chat/completions"));
        assert!(poster
            .headers()
            .iter()
            .any(|(name, value)| name == "Authorization" && value == "Bearer xai-key"));
    });
}

#[test]
fn ollama_mock_prints_extracted_formula() {
    with_isolated_home(|_| {
        let poster = MockPoster::new(ollama_response(FORMULA));
        let formula =
            block_on(suggest_with(Provider::Ollama, &poster, None, None)).expect("suggest");
        assert_eq!(formula, FORMULA);
        assert!(poster.url().contains("127.0.0.1:11434"));
        assert!(poster.url().ends_with("/api/chat"));
        assert_eq!(poster.body()["stream"], json!(false));
        assert!(poster.headers().is_empty());
    });
}

#[test]
fn bedrock_mock_signs_locally_and_prints_formula() {
    with_isolated_home(|_| {
        std::env::set_var("AWS_ACCESS_KEY_ID", "AKIATEST");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "testsecret");
        let poster = MockPoster::new(bedrock_response(FORMULA));
        let formula =
            block_on(suggest_with(Provider::Bedrock, &poster, None, None)).expect("suggest");
        assert_eq!(formula, FORMULA);
        assert!(poster
            .url()
            .contains("bedrock-runtime.us-east-1.amazonaws.com"));
        assert!(poster.url().contains("/converse"));
        assert!(poster
            .headers()
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("authorization")));
    });
}

#[test]
fn suggest_rule_includes_known_identity_paths() {
    with_isolated_home(|home| {
        let contract = home.path().join("contract");
        std::fs::create_dir_all(contract.join("state/parties")).unwrap();
        std::fs::write(contract.join("state/parties/alice.id"), "alice").unwrap();
        let poster = MockPoster::new(openai_response(FORMULA));
        block_on(suggest_with(
            Provider::Openai,
            &poster,
            Some("sk-test"),
            Some(contract.as_path()),
        ))
        .expect("suggest");
        let body = poster.body();
        let system = body["messages"][0]["content"].as_str().unwrap();
        let user = body["messages"][1]["content"].as_str().unwrap();
        assert!(system.contains("Do not invent action names"));
        assert!(!system.contains("prefixed with F1:"));
        assert!(user.contains(providers::SUGGEST_RULE_FEW_SHOT));
        assert!(user.contains("Requirement:\n"));
        assert!(user.contains("/parties/alice.id"));
        assert!(user.ends_with("Formula:\n"));
    });
}

#[test]
fn unconfigured_suggest_rule_fails_with_set_hint() {
    with_isolated_home(|_| {
        let err = block_on(crate::suggest_rule(
            "after this commit either alice or bob must sign",
            None,
            None,
        ))
        .expect_err("unconfigured");
        assert!(format!("{err:#}").contains("modal ai set"));
    });
}

#[test]
fn save_key_rejected_for_bedrock_and_ollama() {
    with_isolated_home(|_| {
        let bedrock = set::Opts::parse_from([
            "set",
            "--provider",
            "bedrock",
            "--save-key",
            "--api-key",
            "secret",
        ]);
        let err = set::run(&bedrock).unwrap_err();
        assert!(format!("{err:#}").contains("--save-key"));

        let ollama = set::Opts::parse_from([
            "set",
            "--provider",
            "ollama",
            "--save-key",
            "--api-key",
            "secret",
        ]);
        let err = set::run(&ollama).unwrap_err();
        assert!(format!("{err:#}").contains("--save-key"));

        let cursor = set::Opts::parse_from([
            "set",
            "--provider",
            "cursor-agent",
            "--save-key",
            "--api-key",
            "secret",
        ]);
        let err = set::run(&cursor).unwrap_err();
        assert!(format!("{err:#}").contains("cursor-agent"));
    });
}

#[test]
fn set_persists_openai_key_only_with_save_key() {
    with_isolated_home(|_| {
        let without_save =
            set::Opts::parse_from(["set", "--provider", "openai", "--api-key", SECRET_KEY]);
        set::run(&without_save).expect("set");
        let loaded = config::load_required().expect("load");
        assert_eq!(loaded.provider, Some(Provider::Openai));
        assert_eq!(loaded.api_key, None);

        let with_save = set::Opts::parse_from([
            "set",
            "--provider",
            "openai",
            "--api-key",
            SECRET_KEY,
            "--save-key",
        ]);
        set::run(&with_save).expect("set save");
        let loaded = config::load_required().expect("load");
        assert_eq!(loaded.api_key.as_deref(), Some(SECRET_KEY));
        let shown = show::format_show(&loaded).expect("show");
        assert!(!shown.contains(SECRET_KEY));
        assert!(shown.contains(&config::redact_key(SECRET_KEY)));
    });
}

#[test]
fn set_persists_cursor_agent_without_key() {
    with_isolated_home(|_| {
        let opts = set::Opts::parse_from(["set", "--provider", "cursor-agent"]);
        set::run(&opts).expect("set");
        let loaded = config::load_required().expect("load");
        assert_eq!(loaded.provider, Some(Provider::CursorAgent));
        assert_eq!(loaded.api_key, None);
        let shown = show::format_show(&loaded).expect("show");
        assert!(shown.contains("Provider: cursor-agent"));
        assert!(shown.contains("agent login"));
    });
}

#[test]
fn cursor_agent_print_invocation_trusts_workspace_and_skips_auto_model() {
    with_isolated_home(|home| {
        std::env::set_var("MODAL_AI_CURSOR_AGENT", home.path().join("fake-agent"));
        let config = AiConfig {
            provider: Some(Provider::CursorAgent),
            ..Default::default()
        };
        let inv = crate::cursor_agent::invocation(
            &config,
            "after this commit either alice or bob must sign",
            None,
            Some(home.path()),
            crate::cursor_agent::SuggestPrintMode::Print,
        )
        .expect("invocation");
        assert!(inv.capture);
        assert!(inv.args.contains(&"-p".to_string()));
        assert!(inv.args.contains(&"--trust".to_string()));
        assert!(inv.args.contains(&"--mode".to_string()));
        assert!(inv.args.contains(&"ask".to_string()));
        assert!(!inv.args.contains(&"--model".to_string()));
        assert_eq!(inv.args[inv.args.len() - 2], "--");
        assert!(inv.args.last().unwrap().contains("after this commit"));
        let workspace = inv
            .args
            .windows(2)
            .find(|pair| pair[0] == "--workspace")
            .map(|pair| pair[1].clone())
            .expect("workspace");
        assert_eq!(
            std::path::PathBuf::from(workspace),
            home.path().canonicalize().unwrap()
        );
    });
}

#[test]
fn cursor_agent_interactive_invocation_omits_print_flags() {
    with_isolated_home(|home| {
        std::env::set_var("MODAL_AI_CURSOR_AGENT", home.path().join("fake-agent"));
        let config = AiConfig {
            provider: Some(Provider::CursorAgent),
            ..Default::default()
        };
        let inv = crate::cursor_agent::invocation(
            &config,
            "after this commit either alice or bob must sign",
            None,
            Some(home.path()),
            crate::cursor_agent::SuggestPrintMode::Interactive,
        )
        .expect("invocation");
        assert!(!inv.capture);
        assert!(!inv.args.contains(&"-p".to_string()));
        assert!(!inv.args.contains(&"--trust".to_string()));
        assert!(inv.args.contains(&"--workspace".to_string()));
    });
}

#[cfg(unix)]
#[test]
fn cursor_agent_print_mode_extracts_formula() {
    with_isolated_home(|home| {
        let bin = home.path().join("fake-agent");
        std::fs::write(&bin, format!("#!/bin/sh\necho '{FORMULA}'\n")).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::env::set_var("MODAL_AI_CURSOR_AGENT", &bin);

        let opts = set::Opts::parse_from(["set", "--provider", "cursor-agent"]);
        set::run(&opts).expect("set");
        let formula = block_on(crate::suggest_rule_mode(
            "after this commit either alice or bob must sign",
            None,
            Some(home.path()),
            crate::cursor_agent::SuggestPrintMode::Print,
        ))
        .expect("suggest");
        assert_eq!(formula, FORMULA);
    });
}
