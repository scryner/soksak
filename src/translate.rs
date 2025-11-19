use crate::config::{AppConfig, FilterConfig};
use crate::llm::{LlmClient, Message};
use crate::transcribe::TranscriptSegment;
use anyhow::Result;
use indicatif::ProgressBar;

#[derive(Debug, Clone, serde::Serialize)]
pub struct TranslatedSegment {
    pub start: i64,
    pub end: i64,
    pub original: String,
    pub translated: String,
}

pub async fn process_translation(
    segments: Vec<TranscriptSegment>,
    app_config: &AppConfig,
    llm_str: &str, // "provider_id/model"
    target_lang: &str,
    window_size: usize,
    filters: Option<&Vec<FilterConfig>>,
    pb: &ProgressBar,
) -> Result<Vec<TranslatedSegment>> {
    let (provider_id, model_name) = llm_str.split_once('/').unwrap_or((llm_str, "default"));

    let provider_config = app_config
        .llm
        .providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| anyhow::anyhow!("Provider {} not found", provider_id))?;

    let client = LlmClient::new(provider_config.clone());

    let mut translated_segments = Vec::new();
    let mut summary = String::from("No context yet.");

    for (i, segment) in segments.iter().enumerate() {
        // 1. Prepare Context
        let start_idx = i.saturating_sub(window_size);
        let end_idx = (i + window_size + 1).min(segments.len());

        let mut context_messages = Vec::new();
        context_messages.push(format!("Summary of previous conversation: {}", summary));

        for j in start_idx..end_idx {
            let seg = &segments[j];
            if i == j {
                context_messages.push(format!("TARGET SENTENCE TO TRANSLATE: {}", seg.text));
            } else {
                context_messages.push(format!("Context: {}", seg.text));
            }
        }

        let context_block = context_messages.join("\n");

        // 2. Translate
        let system_prompt = format!(
            "You are a professional video subtitle translator. Translate the 'TARGET SENTENCE' into {}. \
            Use the provided summary and context to ensure natural flow and correct tone. \
            Output ONLY the translated sentence without quotes or explanations.",
            target_lang
        );

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: system_prompt,
            },
            Message {
                role: "user".to_string(),
                content: context_block.clone(),
            },
        ];

        let translated_text = client.chat_completion(model_name, messages).await?;
        let translated_text = translated_text.trim().to_string();

        // 3. Filter
        let mut keep = true;
        if let Some(filters) = filters {
            for filter in filters {
                let filter_llm_str = filter.llm.as_deref().unwrap_or(llm_str);
                let (f_prov_id, f_model) = filter_llm_str
                    .split_once('/')
                    .unwrap_or((filter_llm_str, "default"));

                let f_client = if f_prov_id == provider_id {
                    // Reuse client if same provider (optimization: could clone or ref, but client creation is cheap)
                    LlmClient::new(provider_config.clone())
                } else {
                    let f_conf = app_config
                        .llm
                        .providers
                        .iter()
                        .find(|p| p.id == f_prov_id)
                        .ok_or_else(|| {
                            anyhow::anyhow!("Filter provider {} not found", f_prov_id)
                        })?;
                    LlmClient::new(f_conf.clone())
                };

                let filter_prompt = format!(
                    "Analyze the following sentence: \"{}\"\n\
                    Condition: {}\n\
                    Return ONLY a probability score between 0.0 and 1.0 that the sentence meets this condition. \
                    Do not output anything else.",
                    translated_text, filter.prompt
                );

                let score_str = f_client
                    .chat_completion(
                        f_model,
                        vec![Message {
                            role: "user".to_string(),
                            content: filter_prompt,
                        }],
                    )
                    .await?;

                // Parse float from response (it might contain text, so use regex or simple parsing)
                let re = regex::Regex::new(r"0\.\d+|1\.0|0|1").unwrap();
                let score: f32 = re
                    .find(&score_str)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(0.0);

                if score > filter.threshold.unwrap_or(0.5) {
                    keep = false;
                    break;
                }
            }
        }

        if keep {
            translated_segments.push(TranslatedSegment {
                start: segment.start,
                end: segment.end,
                original: segment.text.clone(),
                translated: translated_text.clone(),
            });
        }

        // 4. Update Summary
        let summary_prompt = format!(
            "Update the summary of the conversation based on the new sentence.\n\
            Old Summary: {}\n\
            New Sentence: {}\n\
            Output ONLY the updated summary in one or two sentences.",
            summary, segment.text
        );

        // We can use a cheaper model for summary if we want, but let's use the main one for now
        summary = client
            .chat_completion(
                model_name,
                vec![Message {
                    role: "user".to_string(),
                    content: summary_prompt,
                }],
            )
            .await?
            .trim()
            .to_string();

        pb.inc(1);
    }

    Ok(translated_segments)
}
