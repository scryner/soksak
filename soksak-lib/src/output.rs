use crate::transcribe::TranscriptSegment;
use crate::translate::TranslatedSegment;
use anyhow::Result;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn save_transcript_json(path: &Path, segments: &[TranscriptSegment]) -> Result<()> {
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, segments)?;
    Ok(())
}

pub fn save_translation_json(path: &Path, segments: &[TranslatedSegment]) -> Result<()> {
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, segments)?;
    Ok(())
}

pub fn save_srt(path: &Path, segments: &[TranslatedSegment]) -> Result<()> {
    crate::transcribe::validate_timestamps(segments.iter().map(|s| (s.start, s.end)))?;
    let mut file = File::create(path)?;

    for (i, segment) in segments.iter().enumerate() {
        writeln!(file, "{}", i + 1)?;
        writeln!(
            file,
            "{} --> {}",
            format_timestamp(segment.start),
            format_timestamp(segment.end)
        )?;
        writeln!(file, "{}", segment.translated)?;
        writeln!(file)?;
    }

    Ok(())
}

pub fn format_timestamp(cs: i64) -> String {
    let ms = cs * 10;
    let hours = ms / 3600000;
    let minutes = (ms % 3600000) / 60000;
    let seconds = (ms % 60000) / 1000;
    let millis = ms % 1000;

    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, millis)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srt_keeps_refined_centiseconds_and_rejects_invalid_input_before_overwriting() {
        let path = tempfile::NamedTempFile::new().unwrap();
        let mut segments = vec![TranslatedSegment {
            start: 123,
            end: 278,
            original: "Original".into(),
            translated: "번역".into(),
        }];
        save_srt(path.path(), &segments).unwrap();
        let expected = "1\n00:00:01,230 --> 00:00:02,780\n번역\n\n";
        assert_eq!(std::fs::read_to_string(path.path()).unwrap(), expected);
        segments[0].end = 100;
        assert!(save_srt(path.path(), &segments).is_err());
        assert_eq!(std::fs::read_to_string(path.path()).unwrap(), expected);
    }
}
