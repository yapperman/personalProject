use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Ollama,
    Anthropic,
    OpenAI,
}

impl Default for Provider {
    fn default() -> Self {
        Provider::Ollama
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: Provider,
    pub model: String,
    pub api_key: Option<String>,
    /// Ollama base URL; also usable for OpenAI-compatible endpoints
    pub base_url: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: Provider::Ollama,
            model: "qwen3:4b".to_string(),
            api_key: None,
            base_url: Some("http://localhost:11434".to_string()),
        }
    }
}

pub async fn chat(config: &LlmConfig, history: &[ChatMessage]) -> Result<String> {
    match config.provider {
        Provider::Ollama => ollama_chat(config, history).await,
        Provider::Anthropic => anthropic_chat(config, history).await,
        Provider::OpenAI => openai_chat(config, history).await,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

// ── Ollama ────────────────────────────────────────────────────────────────────

async fn ollama_chat(config: &LlmConfig, history: &[ChatMessage]) -> Result<String> {
    let base = config.base_url.as_deref().unwrap_or("http://localhost:11434");
    let url = format!("{}/api/chat", base);

    let body = serde_json::json!({
        "model": config.model,
        "messages": history,
        "stream": false
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Ollama unreachable: {e}"))?;

    if !resp.status().is_success() {
        bail!("Ollama error {}: {}", resp.status(), resp.text().await?);
    }

    let json: serde_json::Value = resp.json().await?;
    Ok(json["message"]["content"]
        .as_str()
        .unwrap_or("(empty response)")
        .to_string())
}

// ── Anthropic ─────────────────────────────────────────────────────────────────

async fn anthropic_chat(config: &LlmConfig, history: &[ChatMessage]) -> Result<String> {
    let api_key = config
        .api_key
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Anthropic API key is not set"))?;

    let body = serde_json::json!({
        "model": config.model,
        "max_tokens": 2048,
        "messages": history
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Anthropic request failed: {e}"))?;

    if !resp.status().is_success() {
        bail!("Anthropic error {}: {}", resp.status(), resp.text().await?);
    }

    let json: serde_json::Value = resp.json().await?;
    Ok(json["content"][0]["text"]
        .as_str()
        .unwrap_or("(empty response)")
        .to_string())
}

// ── OpenAI ────────────────────────────────────────────────────────────────────

async fn openai_chat(config: &LlmConfig, history: &[ChatMessage]) -> Result<String> {
    let api_key = config
        .api_key
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("OpenAI API key is not set"))?;

    let base = config.base_url.as_deref().unwrap_or("https://api.openai.com/v1");
    let url = format!("{}/chat/completions", base);

    let body = serde_json::json!({
        "model": config.model,
        "messages": history
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("OpenAI request failed: {e}"))?;

    if !resp.status().is_success() {
        bail!("OpenAI error {}: {}", resp.status(), resp.text().await?);
    }

    let json: serde_json::Value = resp.json().await?;
    Ok(json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("(empty response)")
        .to_string())
}
