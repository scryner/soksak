use crate::config::{AppConfig, FilterConfig};
use crate::llm::{LlmClient, Message};
use crate::transcribe::TranscriptSegment;
use crate::translate::{BatchItem, BatchTranslationResponse, FilterResponse, TranslatedSegment};
use anyhow::Result;
use indicatif::ProgressBar;

pub async fn process_translation(
    segments: Vec<TranscriptSegment>,
    app_config: &AppConfig,
    llm_str: &str, // "provider_id/model"
    target_lang: &str,
    window_size: usize,
    prepending_system_prompt: &str,
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

    // Process in chunks
    for chunk in segments.chunks(window_size) {
        // The `segments` vector is sequential.

        let batch_items: Vec<BatchItem> = chunk
            .iter()
            .enumerate()
            .map(|(i, seg)| {
                BatchItem {
                    id: i, // Relative ID within the batch
                    text: seg.text.clone(),
                }
            })
            .collect();

        let batch_json = serde_json::to_string(&batch_items)?;

        // 1. Translate Batch
        let system_prompt = format!(
            "{}You are a professional video subtitle translator. Translate the following JSON list of sentences into {}. \
            Maintain the JSON structure with the same 'id' for each item. \
            Use the provided summary to ensure natural flow and correct tone. \
            Output ONLY the JSON response: [{{ \"id\": 0, \"translated_text\": \"...\" }}, ...]",
            prepending_system_prompt, target_lang
        );

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: system_prompt,
            },
            Message {
                role: "user".to_string(),
                content: format!(
                    "Summary of previous conversation: {}\n\nInput JSON:\n{}",
                    summary, batch_json
                ),
            },
        ];

        let response_text = client.chat_completion(model_name, messages).await?;

        // Attempt to parse JSON. If it fails, we might need to repair or fallback.
        // For now, we assume the LLM follows instructions.
        // We might need to strip markdown code blocks if the LLM adds them.
        let clean_response = response_text
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let translations: Vec<BatchTranslationResponse> = match serde_json::from_str(clean_response)
        {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "Failed to parse translation JSON: {}. Response: {}",
                    e, response_text
                );
                // Fallback: return empty translations or original text to avoid crash
                batch_items
                    .iter()
                    .map(|item| BatchTranslationResponse {
                        id: item.id,
                        translated_text: item.text.clone(), // Fallback to original
                    })
                    .collect()
            }
        };

        // Map translations back to segments
        let mut current_batch_results: Vec<TranslatedSegment> = Vec::new();
        for (i, segment) in chunk.iter().enumerate() {
            let translated_text = translations
                .iter()
                .find(|t| t.id == i)
                .map(|t| t.translated_text.clone())
                .unwrap_or_else(|| segment.text.clone()); // Fallback if ID missing

            current_batch_results.push(TranslatedSegment {
                start: segment.start,
                end: segment.end,
                original: segment.text.clone(),
                translated: translated_text,
            });
        }

        // 2. Filter Batch
        if let Some(filters) = filters {
            if !filters.is_empty() {
                // Prepare filter payload
                let filter_items: Vec<BatchItem> = current_batch_results
                    .iter()
                    .enumerate()
                    .map(|(i, seg)| BatchItem {
                        id: i,
                        text: seg.translated.clone(),
                    })
                    .collect();
                let filter_json = serde_json::to_string(&filter_items)?;

                // Construct combined filter prompt
                let mut filter_conditions = String::new();
                for (i, filter) in filters.iter().enumerate() {
                    filter_conditions.push_str(&format!(
                        "- Condition {}: {} (Threshold: {})\n",
                        i + 1,
                        filter.prompt,
                        filter.threshold.unwrap_or(0.5)
                    ));
                }

                let filter_prompt = format!(
                    "Analyze the following JSON list of texts. Identify items that match **ANY** of the following conditions with a confidence probability higher than the specified threshold:\n\
                    {}\n\
                    Return the IDs of items to remove in JSON format: {{ \"remove_ids\": [0, 2, ...] }}. \
                    Output ONLY the JSON.",
                    filter_conditions
                );

                // Use the first filter's provider or default to translation provider
                // The user didn't specify which provider to use for the combined filter,
                // but the plan said "use the provider from the first filter or default to translation provider".
                let filter_llm_str = filters[0].llm.as_deref().unwrap_or(llm_str);
                let (f_prov_id, f_model) = filter_llm_str
                    .split_once('/')
                    .unwrap_or((filter_llm_str, "default"));

                let f_client = if f_prov_id == provider_id {
                    LlmClient::new(provider_config.clone())
                } else {
                    match app_config.llm.providers.iter().find(|p| p.id == f_prov_id) {
                        Some(conf) => LlmClient::new(conf.clone()),
                        None => {
                            eprintln!("Filter provider {} not found, using default", f_prov_id);
                            LlmClient::new(provider_config.clone())
                        }
                    }
                };

                let filter_response_text = f_client
                    .chat_completion(
                        f_model,
                        vec![Message {
                            role: "user".to_string(),
                            content: format!("{}\n\nInput JSON:\n{}", filter_prompt, filter_json),
                        }],
                    )
                    .await?;

                let clean_filter_response = filter_response_text
                    .trim()
                    .trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();

                if let Ok(filter_res) =
                    serde_json::from_str::<FilterResponse>(clean_filter_response)
                {
                    // Remove items
                    let remove_set: std::collections::HashSet<usize> =
                        filter_res.remove_ids.into_iter().collect();
                    let mut filtered_batch = Vec::new();
                    for (i, item) in current_batch_results.into_iter().enumerate() {
                        if !remove_set.contains(&i) {
                            filtered_batch.push(item);
                        }
                    }
                    current_batch_results = filtered_batch;
                } else {
                    eprintln!("Failed to parse filter response: {}", filter_response_text);
                }
            }
        }

        // 3. Update Summary (using the last few translated sentences)
        if !current_batch_results.is_empty() {
            let recent_text = current_batch_results
                .iter()
                .map(|s| s.translated.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            let summary_prompt = format!(
                "Update the summary of the conversation based on the new text.\n\
                Old Summary: {}\n\
                New Text: {}\n\
                Output ONLY the updated summary in one or two sentences.",
                summary, recent_text
            );

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
        }

        translated_segments.extend(current_batch_results);
        pb.inc(chunk.len() as u64);
    }

    Ok(translated_segments)
}
