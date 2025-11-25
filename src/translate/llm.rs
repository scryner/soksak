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
    // Join all texts with ID prefixes
    let batch_text = batch_items
        .iter()
        .map(|item| format!("[{}] {}", item.id, item.text))
        .collect::<Vec<_>>()
        .join("\n");

    let system_prompt = format!(
        "{}You are a professional video subtitle translator. Translate the following text into {}.\n\
        The input text is a list of sentences, each starting with an ID in brackets like `[0]`, `[1]`, etc.\n\
        Translate each line one by one and output the translated text with the SAME ID prefix.\n\
        Example Input:\n\
        [0] Hello world\n\
        [1] How are you?\n\
        Example Output:\n\
        [0] Bonjour le monde\n\
        [1] Comment allez-vous ?\n\
        \n\
        Maintain the exact ID for each line.\n\
        Use the provided summary to ensure natural flow and correct tone.\n\
        Output ONLY the translated text with IDs, no other comments or explanations.",
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
                "Summary of previous conversation: {}\n\nInput Text:\n{}",
                summary, batch_text
            ),
        },
    ];

    let response_text = client
        .chat_completion(model_name, messages, false, None)
        .await?;

    let clean_response = response_text
        .trim()
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Parse response into a map for easy lookup
    let mut translated_map = std::collections::HashMap::new();
    for line in clean_response.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Try to parse "[id] text"
        if let Some(start_bracket) = line.find('[') {
            if let Some(end_bracket) = line[start_bracket..].find(']') {
                let end_bracket_idx = start_bracket + end_bracket;
                if let Ok(id) = line[start_bracket + 1..end_bracket_idx].parse::<usize>() {
                    let text = line[end_bracket_idx + 1..].trim().to_string();
                    translated_map.insert(id, text);
                }
            }
        }
    }

    // Map back to BatchTranslationResponse
    let mut responses = Vec::new();
    for item in batch_items {
        let translated_text = translated_map.remove(&item.id).unwrap_or_else(|| {
            // Fallback if ID not found
            String::new()
        });

        responses.push(BatchTranslationResponse {
            id: item.id,
            translated_text,
        });
    }

    Ok(responses)
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
            false,
            None,
        )
        .await?
        .trim()
        .to_string();

    Ok(new_summary)
}
