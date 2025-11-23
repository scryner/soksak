pub mod apple;
pub mod llm;

use crate::config::{AppConfig, Edit, FilterConfig, Language, Translate, TranslateEngine};
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchItem {
    pub id: usize,
    pub text: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct BatchTranslationResponse {
    pub id: usize,
    pub translated_text: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FilterResponse {
    pub remove_ids: Vec<usize>,
}

pub async fn process_translation(
    source_lang: &Language,
    translate_config: &Translate,
    edit_config: Option<&Edit>,
    segments: Vec<TranscriptSegment>,
    app_config: &AppConfig,
    pb: &ProgressBar,
) -> Result<Vec<TranslatedSegment>> {
    let mut translated_segments = Vec::new();
    let mut summary = String::from("No context yet.");

    // For LLM engine, we need to initialize the client for summary updates
    // We also need a client for the Edit phase if it exists
    // And for filtering if it exists

    // Determine the LLM client for translation if needed
    let (trans_client, trans_model) = match &translate_config.engine {
        TranslateEngine::LLM { model, .. } => {
            let (provider_id, model_name) = model.split_once('/').unwrap_or((model, "default"));
            let provider_config = app_config
                .llm
                .providers
                .iter()
                .find(|p| p.id == provider_id)
                .ok_or_else(|| anyhow::anyhow!("Provider {} not found", provider_id))?;
            (
                Some(LlmClient::new(provider_config.clone())),
                Some(model_name),
            )
        }
        TranslateEngine::Apple { .. } => (None, None),
    };

    // Determine window size
    let window_size = match &translate_config.engine {
        TranslateEngine::LLM { window, .. } => window.unwrap_or(100),
        TranslateEngine::Apple { window } => window.unwrap_or(100),
    };

    // Process in chunks
    for chunk in segments.chunks(window_size) {
        let batch_items: Vec<BatchItem> = chunk
            .iter()
            .enumerate()
            .map(|(i, seg)| BatchItem {
                id: i, // Relative ID within the batch
                text: seg.text.clone(),
            })
            .collect();

        // 1. Translate Batch
        let current_batch_results = match &translate_config.engine {
            TranslateEngine::LLM {
                model: _,
                system_prompt,
                window: _,
            } => {
                if let (Some(client), Some(model)) = (&trans_client, trans_model) {
                    llm::translate_batch(
                        client,
                        model,
                        &batch_items,
                        &translate_config.target_lang.to_string(),
                        system_prompt.as_deref().unwrap_or(""),
                        &summary,
                    )
                    .await?
                } else {
                    unreachable!("LLM client should be initialized for LLM engine");
                }
            }
            TranslateEngine::Apple { .. } => {
                let source_lang = match source_lang {
                    Language::Auto => None,
                    _ => Some(source_lang.to_string()),
                };

                apple::translate_batch(
                    &batch_items,
                    source_lang.as_deref(),
                    &translate_config.target_lang.to_string(),
                )
                .await?
            }
        };

        // Map translations back to segments to preserve start/end times
        let mut mapped_results: Vec<TranslatedSegment> = Vec::new();
        for (i, segment) in chunk.iter().enumerate() {
            let translated_text = current_batch_results
                .iter()
                .find(|t| t.id == i)
                .map(|t| t.translated_text.clone())
                .unwrap_or_else(|| segment.text.clone());

            mapped_results.push(TranslatedSegment {
                start: segment.start,
                end: segment.end,
                original: segment.text.clone(),
                translated: translated_text,
            });
        }

        // 2. Edit Batch (Refinement)
        if let Some(edit) = edit_config {
            if let Some(instructions) = &edit.instructions {
                if !instructions.is_empty() {
                    mapped_results = edit_batch(
                        mapped_results,
                        edit,
                        app_config,
                        &translate_config.target_lang,
                    )
                    .await?;
                }
            }
        }

        // 3. Filter Batch
        // Check if filtering is configured in Edit config
        if let Some(edit) = edit_config {
            if let Some(filters) = &edit.filters {
                if !filters.is_empty() {
                    // We need a default LLM for filtering. Use edit.default_model
                    mapped_results =
                        filter_batch(mapped_results, filters, app_config, &edit.default_model)
                            .await?;
                }
            }
        }

        // 4. Update Summary (Only for LLM engine usually)
        if let (Some(client), Some(model)) = (&trans_client, trans_model) {
            if !mapped_results.is_empty() {
                summary = llm::update_summary(client, model, &summary, &mapped_results).await?;
            }
        }

        translated_segments.extend(mapped_results);
        pb.inc(chunk.len() as u64);
    }

    Ok(translated_segments)
}

async fn edit_batch(
    batch: Vec<TranslatedSegment>,
    edit_config: &Edit,
    app_config: &AppConfig,
    target_lang: &Language,
) -> Result<Vec<TranslatedSegment>> {
    let (provider_id, model_name) = edit_config
        .default_model
        .split_once('/')
        .unwrap_or((&edit_config.default_model, "default"));

    let provider_config = app_config
        .llm
        .providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| anyhow::anyhow!("Provider {} not found", provider_id))?;
    let client = LlmClient::new(provider_config.clone());

    // Join all texts with newlines
    let batch_text = batch
        .iter()
        .map(|seg| seg.translated.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let instructions_str = edit_config
        .instructions
        .as_ref()
        .map(|v| v.join("\n"))
        .unwrap_or_default();

    let system_prompt = format!(
        "You are a professional editor. Refine the following translated sentences based on these instructions:\n\
        Target Language: {}\n\
        Instructions:\n\
        {}\n\
        DO NOT TRANSLATE THE TEXT. JUST EDIT THE TEXT BASED ON THE INSTRUCTIONS.\n\
        The input text is a list of sentences separated by newlines.\n\
        Edit each line one by one and output the refined text separated by newlines.\n\
        IMPORTANT: You must maintain the exact line-by-line correspondence. Line N of the output must be the edited version of Line N of the input.\n\
        Do not merge, split, or reorder lines.\n\
        Output ONLY the refined text, no other comments or explanations.",
        target_lang, instructions_str
    );

    let messages = vec![
        Message {
            role: "system".to_string(),
            content: system_prompt,
        },
        Message {
            role: "user".to_string(),
            content: format!("Input Text:\n{}", batch_text),
        },
    ];

    let curl_cmd = client.get_curl_command(model_name, &messages, false, None);
    log::debug!("CURL: {}", curl_cmd);

    let response_text = client
        .chat_completion(model_name, messages, false, None)
        .await?;

    let clean_response = response_text
        .trim()
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let refined_lines: Vec<&str> = clean_response.lines().collect();

    // Map back
    let mut refined_batch = Vec::new();
    for (i, segment) in batch.into_iter().enumerate() {
        let refined_text = if i < refined_lines.len() {
            refined_lines[i].trim().to_string()
        } else {
            // Fallback to unedited if missing
            segment.translated.clone()
        };

        refined_batch.push(TranslatedSegment {
            translated: refined_text,
            ..segment
        });
    }

    Ok(refined_batch)
}

async fn filter_batch(
    batch: Vec<TranslatedSegment>,
    filters: &Vec<FilterConfig>,
    app_config: &AppConfig,
    default_llm_str: &str,
) -> Result<Vec<TranslatedSegment>> {
    // Prepare filter payload
    let filter_items: Vec<BatchItem> = batch
        .iter()
        .enumerate()
        .map(|(i, seg)| BatchItem {
            id: i,
            text: seg.translated.clone(),
        })
        .collect();
    let filter_json = serde_json::to_string(&filter_items)?;

    // Construct combined filter prompt
    // Construct combined filter prompt
    let mut filter_instructions = String::new();
    for (i, filter) in filters.iter().enumerate() {
        filter_instructions.push_str(&format!(
            "{}. {} (Confidence Threshold: {})\n",
            i + 1,
            filter.prompt,
            filter.threshold.unwrap_or(0.7)
        ));
    }

    let filter_prompt = format!(
        "You are a content filter. Analyze the following JSON list of texts and identify items that should be removed based on the following instructions:\n\
        {}\n\
        For each item, if it matches any of the instructions with a confidence score higher than the specified threshold, mark it for removal.\n\
        Return the IDs of items to remove in JSON format: {{ \"remove_ids\": [0, 2, ...] }}. \
        Output ONLY the JSON.",
        filter_instructions
    );

    // Determine provider for filtering
    let filter_llm_str = filters[0].llm.as_deref().unwrap_or(default_llm_str);
    let (f_prov_id, f_model) = filter_llm_str
        .split_once('/')
        .unwrap_or((filter_llm_str, "default"));

    let f_client = if let Some(p) = app_config.llm.providers.iter().find(|p| p.id == f_prov_id) {
        LlmClient::new(p.clone())
    } else {
        // Fallback to default provider if specific filter provider not found
        // We need to find the default provider config again
        let (def_prov_id, _) = default_llm_str
            .split_once('/')
            .unwrap_or((default_llm_str, "default"));
        let def_conf = app_config
            .llm
            .providers
            .iter()
            .find(|p| p.id == def_prov_id)
            .ok_or_else(|| anyhow::anyhow!("Default provider {} not found", def_prov_id))?;
        LlmClient::new(def_conf.clone())
    };

    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "remove_ids": {
                "type": "array",
                "items": { "type": "integer" }
            }
        },
        "required": ["remove_ids"],
        "additionalProperties": false
    });

    let filter_response_text = f_client
        .chat_completion(
            f_model,
            vec![Message {
                role: "user".to_string(),
                content: format!("{}\n\nInput JSON:\n{}", filter_prompt, filter_json),
            }],
            true,
            Some(schema),
        )
        .await?;

    let clean_filter_response = filter_response_text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Ok(filter_res) = serde_json::from_str::<FilterResponse>(clean_filter_response) {
        let remove_set: std::collections::HashSet<usize> =
            filter_res.remove_ids.into_iter().collect();
        let mut filtered_batch = Vec::new();
        for (i, item) in batch.into_iter().enumerate() {
            if !remove_set.contains(&i) {
                filtered_batch.push(item);
            }
        }
        Ok(filtered_batch)
    } else {
        eprintln!("Failed to parse filter response: {}", filter_response_text);
        Ok(batch) // Return original if parsing fails
    }
}
