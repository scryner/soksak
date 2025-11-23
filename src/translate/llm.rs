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
    // Join all texts with newlines
    let batch_text = batch_items
        .iter()
        .map(|item| item.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let system_prompt = format!(
        "{}You are a professional video subtitle translator. Translate the following text into {}.\n\
        The input text is a list of sentences separated by newlines.\n\
        Translate each line one by one and output the translated text separated by newlines.\n\
        Maintain the same number of lines as the input.\n\
        Use the provided summary to ensure natural flow and correct tone.\n\
        Output ONLY the translated text, no other comments or explanations.",
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

    let translated_lines: Vec<&str> = clean_response.lines().collect();

    // Map back to BatchTranslationResponse
    let mut responses = Vec::new();
    for (i, item) in batch_items.iter().enumerate() {
        let translated_text = if i < translated_lines.len() {
            translated_lines[i].trim().to_string()
        } else {
            // Fallback if LLM output fewer lines
            String::new()
        };

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
