use crate::llm::{LlmClient, Message};
use crate::translate::{BatchItem, BatchTranslationResponse, TranslatedSegment};
use anyhow::Result;

pub async fn translate_batch(
    client: &LlmClient,
    model_name: &str,
    batch_items: &[BatchItem],
    target_lang: &str,
    prepending_system_prompt: &str,
    summary: &str,
) -> Result<Vec<BatchTranslationResponse>> {
    let batch_json = serde_json::to_string(batch_items)?;

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

    let clean_response = response_text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    match serde_json::from_str::<Vec<BatchTranslationResponse>>(clean_response) {
        Ok(t) => Ok(t),
        Err(e) => {
            eprintln!(
                "Failed to parse translation JSON: {}. Response: {}",
                e, response_text
            );
            // Fallback: return original text
            Ok(batch_items
                .iter()
                .map(|item| BatchTranslationResponse {
                    id: item.id,
                    translated_text: item.text.clone(),
                })
                .collect())
        }
    }
}

pub async fn update_summary(
    client: &LlmClient,
    model_name: &str,
    current_summary: &str,
    recent_segments: &[TranslatedSegment],
) -> Result<String> {
    let recent_text = recent_segments
        .iter()
        .map(|s| s.translated.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    let summary_prompt = format!(
        "Update the summary of the conversation based on the new text.\n\
        Old Summary: {}\n\
        New Text: {}\n\
        Output ONLY the updated summary in one or two sentences.",
        current_summary, recent_text
    );

    let new_summary = client
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

    Ok(new_summary)
}
