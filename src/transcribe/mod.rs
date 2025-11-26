pub mod whisper_cpp;
#[cfg(feature = "apple")]
pub mod whisperkit;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranscriptSegment {
    pub start: i64, // centiseconds
    pub end: i64,   // centiseconds
    pub text: String,
}
