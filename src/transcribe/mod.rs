pub mod whisper_cpp;
pub mod whisperkit;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranscriptSegment {
    pub start: i64, // milliseconds
    pub end: i64,   // milliseconds
    pub text: String,
}
