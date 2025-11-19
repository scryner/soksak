use crate::transcribe::TranscriptSegment;
use crate::translate::TranslatedSegment;
use anyhow::Result;
use std::path::Path;
use std::fs::File;
use std::io::Write;

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
    let mut file = File::create(path)?;
    
    for (i, segment) in segments.iter().enumerate() {
        writeln!(file, "{}", i + 1)?;
        writeln!(file, "{} --> {}", format_timestamp(segment.start), format_timestamp(segment.end))?;
        writeln!(file, "{}", segment.translated)?;
        writeln!(file)?;
    }
    
    Ok(())
}

fn format_timestamp(ms: i64) -> String {
    let hours = ms / 3600000;
    let minutes = (ms % 3600000) / 60000;
    let seconds = (ms % 60000) / 1000;
    let millis = ms % 1000;
    
    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, millis)
}
