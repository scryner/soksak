pub mod llm;

#[derive(Debug, Clone, serde::Serialize)]
pub struct TranslatedSegment {
    pub start: i64,
    pub end: i64,
    pub original: String,
    pub translated: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BatchItem {
    id: usize,
    text: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct BatchTranslationResponse {
    id: usize,
    translated_text: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct FilterResponse {
    remove_ids: Vec<usize>,
}
