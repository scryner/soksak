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

    pub async fn chat_completion(
        &self,
        model: &str,
        messages: Vec<Message>,
        json_mode: bool,
        response_schema: Option<serde_json::Value>,
    ) -> Result<String> {
        match self.provider.api_type {
            crate::config::ApiType::OpenAI | crate::config::ApiType::Ollama => {
                self.chat_completion_openai(model, messages, json_mode, response_schema)
                    .await
            }
            crate::config::ApiType::Claude => {
                self.chat_completion_claude(model, messages, json_mode, response_schema)
                    .await
            }
            crate::config::ApiType::Gemini => {
                self.chat_completion_gemini(model, messages, json_mode, response_schema)
                    .await
            }
        }
    }

    async fn chat_completion_openai(
        &self,
        model: &str,
        messages: Vec<Message>,
        json_mode: bool,
        response_schema: Option<serde_json::Value>,
    ) -> Result<String> {
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

        let mut body = json!({
            "model": model,
            "messages": messages,
        });

        if json_mode {
            match self.provider.json_mode_type {
                crate::config::JsonModeType::JsonObject => {
                    if matches!(self.provider.api_type, crate::config::ApiType::Ollama) {
                        body.as_object_mut()
                            .unwrap()
                            .insert("format".to_string(), json!("json"));
                    } else {
                        body.as_object_mut().unwrap().insert(
                            "response_format".to_string(),
                            json!({ "type": "json_object" }),
                        );
                    }
                }
                crate::config::JsonModeType::JsonSchema => {
                    if let Some(schema) = response_schema {
                        body.as_object_mut().unwrap().insert(
                            "response_format".to_string(),
                            json!({
                                "type": "json_schema",
                                "json_schema": {
                                    "name": "response",
                                    "strict": true,
                                    "schema": schema
                                }
                            }),
                        );
                    } else {
                        // Fallback if no schema provided but mode is JsonSchema
                        // We can't really do strict schema without a schema, so maybe fallback to json_object?
                        // Or just warn and proceed.
                        eprintln!(
                            "Warning: JsonSchema mode enabled but no schema provided. Falling back to json_object."
                        );
                        body.as_object_mut().unwrap().insert(
                            "response_format".to_string(),
                            json!({ "type": "json_object" }),
                        );
                    }
                }
                crate::config::JsonModeType::None => {
                    // Do nothing, rely on prompt
                }
            }
        }

        let mut current_body = body;
        let mut retry_count = 0;

        loop {
            let mut request = self.client.post(&url).json(&current_body);

            if let Some(api_key) = &self.provider.api_key {
                request = request.header("Authorization", format!("Bearer {}", api_key));
            }

            let response = request.send().await?;

            if response.status().is_success() {
                let response_json: serde_json::Value = response.json().await?;
                let content = response_json["choices"][0]["message"]["content"]
                    .as_str()
                    .context("Failed to parse LLM response content")?
                    .to_string();
                return Ok(content);
            } else {
                let error_text = response.text().await?;
                // Check if the error is about response_format not supporting json_object
                // Error example: "'response_format.type' must be 'json_schema' or 'text'"
                if retry_count == 0
                    && error_text.contains("response_format")
                    && error_text.contains("json_schema")
                {
                    eprintln!(
                        "Warning: Provider does not support 'json_object'. Retrying without response_format..."
                    );
                    if let Some(obj) = current_body.as_object_mut() {
                        obj.remove("response_format");
                    }
                    retry_count += 1;
                    continue;
                }
                anyhow::bail!("LLM API error: {}", error_text);
            }
        }
    }

    async fn chat_completion_claude(
        &self,
        model: &str,
        messages: Vec<Message>,
        _json_mode: bool,
        _response_schema: Option<serde_json::Value>,
    ) -> Result<String> {
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

    async fn chat_completion_gemini(
        &self,
        model: &str,
        messages: Vec<Message>,
        json_mode: bool,
        _response_schema: Option<serde_json::Value>,
    ) -> Result<String> {
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

        let mut body = json!({
            "contents": contents,
        });

        if json_mode {
            // Gemini doesn't have the same JsonModeType enum logic yet, but we can apply it if we want.
            // For now, let's assume Gemini always supports responseMimeType if json_mode is true,
            // unless we want to support None for Gemini too.
            // Let's apply the same logic for consistency.
            match self.provider.json_mode_type {
                crate::config::JsonModeType::None => {}
                _ => {
                    body.as_object_mut().unwrap().insert(
                        "generationConfig".to_string(),
                        json!({ "responseMimeType": "application/json" }),
                    );
                }
            }
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ApiType, JsonModeType, LlmProviderConfig};

    #[tokio::test]
    #[ignore] // Ignored by default, run explicitly to test local LLM
    async fn test_lmstudio_json_mode() {
        let provider = LlmProviderConfig {
            id: "lmstudio".to_string(),
            base_url: Some("http://localhost:1234".to_string()),
            api_key: Some("lm-studio".to_string()),
            api_type: ApiType::OpenAI,
            json_mode_type: JsonModeType::None, // Explicitly disable JSON mode to fix the error
        };

        let client = LlmClient::new(provider);
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: "You are a helpful assistant. Output JSON.".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: "Say hello in JSON format: {\"message\": \"hello\"}".to_string(),
            },
        ];

        // This should succeed now (or fail with model error, but NOT response_format error)
        let result = client
            .chat_completion("gpt-oss-120b", messages, true, None)
            .await;

        match result {
            Ok(res) => println!("Success: {}", res),
            Err(e) => {
                println!("Error: {}", e);
                if e.to_string().contains("response_format.type") {
                    panic!("Failed: Still getting response_format error");
                }
            }
        }
    }
}
