pub mod alignment;
mod timing;
pub mod whisper_cpp;
#[cfg(feature = "apple")]
pub mod whisperkit;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranscriptSegment {
    pub start: i64, // centiseconds
    pub end: i64,   // centiseconds
    pub text: String,
}

/// Reject invalid ranges and order reversals without silently clipping spoken words.
/// Existing overlap is allowed (for example, simultaneous speakers).
pub fn validate_timestamps(ranges: impl IntoIterator<Item = (i64, i64)>) -> anyhow::Result<()> {
    let mut previous_start = 0;
    for (id, (start, end)) in ranges.into_iter().enumerate() {
        if start < 0 || end <= start || start < previous_start || end > i64::MAX / 10 {
            anyhow::bail!("Invalid timestamps at segment {id}: {start}..{end} centiseconds");
        }
        previous_start = start;
    }
    Ok(())
}
