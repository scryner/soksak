use crate::config::LlmProviderConfig;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub struct LlmClient {
    client: reqwest::Client,
    provider: LlmProviderConfig,
}

impl LlmClient {
    pub fn new(provider: LlmProviderConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            provider,
        }
    }

    pub async fn chat_completion(&self, model: &str, messages: Vec<Message>) -> Result<String> {
        match self.provider.api_type {
            crate::config::ApiType::OpenAI | crate::config::ApiType::Ollama => {
                self.chat_completion_openai(model, messages).await
            }
            crate::config::ApiType::Claude => self.chat_completion_claude(model, messages).await,
            crate::config::ApiType::Gemini => self.chat_completion_gemini(model, messages).await,
        }
    }

    async fn chat_completion_openai(&self, model: &str, messages: Vec<Message>) -> Result<String> {
        let default_url = if matches!(self.provider.api_type, crate::config::ApiType::Ollama) {
            "http://localhost:11434/v1/chat/completions"
        } else {
            "https://api.openai.com/v1/chat/completions"
        };

        let url = if let Some(base_url) = &self.provider.base_url {
            format!("{}/v1/chat/completions", base_url.trim_end_matches('/'))
        } else {
            default_url.to_string()
        };

        let body = json!({
            "model": model,
            "messages": messages,
        });

        let mut request = self.client.post(&url).json(&body);

        if let Some(api_key) = &self.provider.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("LLM API error: {}", error_text);
        }

        let response_json: serde_json::Value = response.json().await?;

        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .context("Failed to parse LLM response content")?
            .to_string();

        Ok(content)
    }

    async fn chat_completion_claude(&self, model: &str, messages: Vec<Message>) -> Result<String> {
        let url = if let Some(base_url) = &self.provider.base_url {
            format!("{}/v1/messages", base_url.trim_end_matches('/'))
        } else {
            "https://api.anthropic.com/v1/messages".to_string()
        };

        let body = json!({
            "model": model,
            "messages": messages,
            "max_tokens": 4096, // Reasonable default
        });

        let request = self
            .client
            .post(&url)
            .header(
                "x-api-key",
                self.provider.api_key.as_deref().unwrap_or_default(),
            )
            .header("anthropic-version", "2023-06-01")
            .json(&body);

        let response = request.send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Claude API error: {}", error_text);
        }

        let response_json: serde_json::Value = response.json().await?;

        let content = response_json["content"][0]["text"]
            .as_str()
            .context("Failed to parse Claude response content")?
            .to_string();

        Ok(content)
    }

    async fn chat_completion_gemini(&self, model: &str, messages: Vec<Message>) -> Result<String> {
        let base_url = self
            .provider
            .base_url
            .as_deref()
            .unwrap_or("https://generativelanguage.googleapis.com");
        let api_key = self
            .provider
            .api_key
            .as_deref()
            .context("API key is required for Gemini")?;

        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            base_url.trim_end_matches('/'),
            model,
            api_key
        );

        // Convert messages to Gemini format
        let contents: Vec<serde_json::Value> = messages
            .iter()
            .map(|msg| {
                let role = if msg.role == "user" { "user" } else { "model" };
                json!({
                    "role": role,
                    "parts": [{ "text": msg.content }]
                })
            })
            .collect();

        let body = json!({
            "contents": contents,
        });

        let request = self.client.post(&url).json(&body);
        let response = request.send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Gemini API error: {}", error_text);
        }

        let response_json: serde_json::Value = response.json().await?;

        let content = response_json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .context("Failed to parse Gemini response content")?
            .to_string();

        Ok(content)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}
