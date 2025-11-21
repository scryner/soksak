pub mod apple;
pub mod llm;

use crate::config::{AppConfig, Edit, FilterConfig, Translate, TranslateEngine};
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
                apple::translate_batch(&batch_items, &translate_config.target_lang.to_string())
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
                    mapped_results = edit_batch(mapped_results, edit, app_config).await?;
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

    let batch_items: Vec<BatchItem> = batch
        .iter()
        .enumerate()
        .map(|(i, seg)| BatchItem {
            id: i,
            text: seg.translated.clone(),
        })
        .collect();
    let batch_json = serde_json::to_string(&batch_items)?;

    let instructions_str = edit_config
        .instructions
        .as_ref()
        .map(|v| v.join("\n"))
        .unwrap_or_default();

    let system_prompt = format!(
        "You are a professional editor. Refine the following translated sentences based on these instructions:\n\
        {}\n\
        Maintain the JSON structure with the same 'id' for each item.\n\
        Output ONLY the JSON response: [{{ \"id\": 0, \"translated_text\": \"...\" }}, ...]",
        instructions_str
    );

    let messages = vec![
        Message {
            role: "system".to_string(),
            content: system_prompt,
        },
        Message {
            role: "user".to_string(),
            content: format!("Input JSON:\n{}", batch_json),
        },
    ];

    let response_text = client.chat_completion(model_name, messages).await?;

    let clean_response = response_text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let refined_items: Vec<BatchTranslationResponse> = match serde_json::from_str(clean_response) {
        Ok(t) => t,
        Err(e) => {
            eprintln!(
                "Failed to parse edit JSON: {}. Response: {}",
                e, response_text
            );
            return Ok(batch); // Fallback to unedited
        }
    };

    // Map back
    let mut refined_batch = Vec::new();
    for (i, segment) in batch.into_iter().enumerate() {
        let refined_text = refined_items
            .iter()
            .find(|t| t.id == i)
            .map(|t| t.translated_text.clone())
            .unwrap_or(segment.translated); // Fallback to unedited if missing

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
