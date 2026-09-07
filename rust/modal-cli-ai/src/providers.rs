use anyhow::{anyhow, bail, Context, Result};
use aws_credential_types::Credentials;
use aws_sigv4::http_request::{sign, SignableBody, SignableRequest, SigningSettings};
use aws_sigv4::sign::v4;
use aws_smithy_runtime_api::client::identity::Identity;
use serde_json::{json, Value};
use std::time::{Duration, SystemTime};

use crate::config::{AiConfig, Provider};

pub const SUGGEST_RULE_INSTRUCTIONS: &str = r#"Reply with a single Modality formula only. No markdown, no labels, no explanation.
The formula is the inner contents of a rule `formula { ... }` block, suitable as the argument to `modal c add-rule`.
When the request is about commits after the current one, use `[] always(...)`.
When the request names Alice or Bob, use `/parties/alice.id` and `/parties/bob.id`."#;

pub fn suggest_rule_system_prompt() -> String {
    format!(
        "{}\n\n{}",
        modality_lang::llm_synthesis::SYSTEM_PROMPT,
        SUGGEST_RULE_INSTRUCTIONS
    )
}

pub fn extract_formula(response: &str) -> Result<String> {
    let formulas = modality_lang::llm_synthesis::parse_llm_response(response);
    formulas
        .into_iter()
        .next()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            anyhow!("Could not parse a Modality formula from the model response:\n{response}")
        })
}

#[async_trait::async_trait]
pub trait JsonPoster: Send + Sync {
    async fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &Value,
    ) -> Result<Value>;
}

pub struct ReqwestPoster {
    client: reqwest::Client,
}

impl ReqwestPoster {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()?,
        })
    }
}

#[async_trait::async_trait]
impl JsonPoster for ReqwestPoster {
    async fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &Value,
    ) -> Result<Value> {
        let mut request = self.client.post(url).json(body);
        for (name, value) in headers {
            request = request.header(name.as_str(), value.as_str());
        }
        let response = request
            .send()
            .await
            .with_context(|| format!("POST {url}"))?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            bail!("AI provider returned HTTP {status}: {text}");
        }
        serde_json::from_str(&text)
            .with_context(|| format!("AI provider returned non-JSON response: {text}"))
    }
}

pub async fn complete(
    config: &AiConfig,
    user_prompt: &str,
    api_key: Option<&str>,
    poster: &dyn JsonPoster,
) -> Result<String> {
    let system = suggest_rule_system_prompt();
    let provider = config.provider()?;
    let model = config.model()?;
    match provider {
        Provider::Openai | Provider::Grok => {
            complete_openai_compatible(config, &model, &system, user_prompt, api_key, poster).await
        }
        Provider::Anthropic => {
            complete_anthropic(config, &model, &system, user_prompt, api_key, poster).await
        }
        Provider::Ollama => complete_ollama(config, &model, &system, user_prompt, poster).await,
        Provider::Bedrock => complete_bedrock(config, &model, &system, user_prompt, poster).await,
    }
}

async fn complete_openai_compatible(
    config: &AiConfig,
    model: &str,
    system: &str,
    user_prompt: &str,
    api_key: Option<&str>,
    poster: &dyn JsonPoster,
) -> Result<String> {
    let key = config.require_api_key(api_key)?;
    let base = config
        .base_url()?
        .ok_or_else(|| anyhow!("missing base URL"))?;
    let url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));
    let body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user_prompt}
        ]
    });
    let headers = vec![("Authorization".to_string(), format!("Bearer {key}"))];
    let value = poster.post_json(&url, &headers, &body).await?;
    openai_text(&value)
}

async fn complete_anthropic(
    config: &AiConfig,
    model: &str,
    system: &str,
    user_prompt: &str,
    api_key: Option<&str>,
    poster: &dyn JsonPoster,
) -> Result<String> {
    let key = config.require_api_key(api_key)?;
    let base = config
        .base_url()?
        .ok_or_else(|| anyhow!("missing base URL"))?;
    let url = format!("{}/v1/messages", base.trim_end_matches('/'));
    let body = json!({
        "model": model,
        "max_tokens": 1024,
        "system": system,
        "messages": [{"role": "user", "content": user_prompt}]
    });
    let headers = vec![
        ("x-api-key".to_string(), key),
        ("anthropic-version".to_string(), "2023-06-01".to_string()),
    ];
    let value = poster.post_json(&url, &headers, &body).await?;
    anthropic_text(&value)
}

async fn complete_ollama(
    config: &AiConfig,
    model: &str,
    system: &str,
    user_prompt: &str,
    poster: &dyn JsonPoster,
) -> Result<String> {
    let base = config
        .base_url()?
        .ok_or_else(|| anyhow!("missing base URL"))?;
    let url = format!("{}/api/chat", base.trim_end_matches('/'));
    let body = json!({
        "model": model,
        "stream": false,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user_prompt}
        ]
    });
    let value = poster.post_json(&url, &[], &body).await?;
    ollama_text(&value)
}

async fn complete_bedrock(
    config: &AiConfig,
    model: &str,
    system: &str,
    user_prompt: &str,
    poster: &dyn JsonPoster,
) -> Result<String> {
    let region = config.region();
    let encoded_model =
        percent_encoding::utf8_percent_encode(model, percent_encoding::NON_ALPHANUMERIC);
    let url =
        format!("https://bedrock-runtime.{region}.amazonaws.com/model/{encoded_model}/converse");
    let body = json!({
        "system": [{"text": system}],
        "messages": [{"role": "user", "content": [{"text": user_prompt}]}],
        "inferenceConfig": {"maxTokens": 1024}
    });
    let headers = sign_bedrock_headers(&url, &body, &region)?;
    let value = poster.post_json(&url, &headers, &body).await?;
    bedrock_text(&value)
}

fn sign_bedrock_headers(url: &str, body: &Value, region: &str) -> Result<Vec<(String, String)>> {
    let credentials = aws_env_credentials()?;
    let identity: Identity = credentials.into();
    let signing_params = v4::SigningParams::builder()
        .identity(&identity)
        .region(region)
        .name("bedrock")
        .time(SystemTime::now())
        .settings(SigningSettings::default())
        .build()
        .map_err(|err| anyhow!("Failed to build Bedrock signing params: {err}"))?;
    let body_bytes = serde_json::to_vec(body)?;
    let signable = SignableRequest::new(
        "POST",
        url,
        std::iter::once(("content-type", "application/json")),
        SignableBody::Bytes(&body_bytes),
    )
    .map_err(|err| anyhow!("Failed to build Bedrock signable request: {err}"))?;
    let (instructions, _signature) = sign(signable, &signing_params.into())
        .map_err(|err| anyhow!("Failed to sign Bedrock request: {err}"))?
        .into_parts();
    let mut headers = Vec::new();
    for (name, value) in instructions.headers() {
        headers.push((name.to_string(), value.to_string()));
    }
    headers.push(("Content-Type".to_string(), "application/json".to_string()));
    Ok(headers)
}

fn aws_env_credentials() -> Result<Credentials> {
    let access = std::env::var("AWS_ACCESS_KEY_ID")
        .map_err(|_| anyhow!("Bedrock requires AWS credentials. Set AWS_ACCESS_KEY_ID."))?;
    let secret = std::env::var("AWS_SECRET_ACCESS_KEY")
        .map_err(|_| anyhow!("Bedrock requires AWS credentials. Set AWS_SECRET_ACCESS_KEY."))?;
    let token = std::env::var("AWS_SESSION_TOKEN").ok();
    Ok(Credentials::new(
        access,
        secret,
        token,
        None,
        "modal-cli-ai",
    ))
}

pub fn openai_text(value: &Value) -> Result<String> {
    value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("OpenAI-compatible response missing choices[0].message.content"))
}

pub fn anthropic_text(value: &Value) -> Result<String> {
    value
        .pointer("/content/0/text")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("Anthropic response missing content[0].text"))
}

pub fn ollama_text(value: &Value) -> Result<String> {
    value
        .pointer("/message/content")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("Ollama response missing message.content"))
}

pub fn bedrock_text(value: &Value) -> Result<String> {
    value
        .pointer("/output/message/content/0/text")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("Bedrock Converse response missing output.message.content[0].text"))
}
