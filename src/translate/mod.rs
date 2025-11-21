pub mod apple;
pub mod llm;

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

#[derive(Debug, Clone, Copy)]
pub enum TranslateEngine {
    LLM,
    Apple,
}

pub async fn process_translation(
    engine: TranslateEngine,
    segments: Vec<TranscriptSegment>,
    app_config: &AppConfig,
    llm_str: &str, // "provider_id/model"
    target_lang: &str,
    window_size: usize,
    prepending_system_prompt: &str,
    filters: Option<&Vec<FilterConfig>>,
    pb: &ProgressBar,
) -> Result<Vec<TranslatedSegment>> {
    let mut translated_segments = Vec::new();
    let mut summary = String::from("No context yet.");

    // For LLM engine, we need to initialize the client for summary updates
    let (provider_id, model_name) = llm_str.split_once('/').unwrap_or((llm_str, "default"));
    let provider_config = app_config
        .llm
        .providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| anyhow::anyhow!("Provider {} not found", provider_id))?;
    let llm_client = LlmClient::new(provider_config.clone());

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
        let current_batch_results = match engine {
            TranslateEngine::LLM => {
                llm::translate_batch(
                    &llm_client,
                    model_name,
                    &batch_items,
                    target_lang,
                    prepending_system_prompt,
                    &summary,
                )
                .await?
            }
            TranslateEngine::Apple => apple::translate_batch(&batch_items, target_lang).await?,
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

        // 2. Filter Batch
        if let Some(filters) = filters {
            if !filters.is_empty() {
                mapped_results = filter_batch(
                    mapped_results,
                    filters,
                    app_config,
                    llm_str, // Use the main LLM for filtering if not specified
                )
                .await?;
            }
        }

        // 3. Update Summary (Only for LLM engine usually, but good to keep context if we switch engines or for hybrid)
        if matches!(engine, TranslateEngine::LLM) && !mapped_results.is_empty() {
            summary =
                llm::update_summary(&llm_client, model_name, &summary, &mapped_results).await?;
        }

        translated_segments.extend(mapped_results);
        pb.inc(chunk.len() as u64);
    }

    Ok(translated_segments)
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
