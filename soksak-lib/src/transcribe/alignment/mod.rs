use crate::{
    config::{AlignmentConfig, AlignmentMode},
    ffmpeg_decoder::ExtractedAudio,
    progress::Progress,
    transcribe::{validate_timestamps, TranscriptSegment},
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use tempfile::{NamedTempFile, TempDir};

#[derive(Debug, Serialize)]
pub struct TimingReport {
    pub version: u32,
    pub time_unit: &'static str,
    pub language: String,
    pub audio_stream: u32,
    pub timeline_origin_seconds: f64,
    pub duration_cs: i64,
    pub alignment_status: String,
    pub segments: Vec<TimingChange>,
}
#[derive(Debug, Serialize)]
pub struct TimingChange {
    pub id: usize,
    pub original_start: i64,
    pub original_end: i64,
    pub start: i64,
    pub end: i64,
    pub status: String,
    pub score: Option<f64>,
    pub coverage: Option<f64>,
}
pub struct RefinedTranscript {
    pub segments: Vec<TranscriptSegment>,
    pub report: TimingReport,
}
#[derive(Debug, Deserialize)]
struct Candidate {
    id: usize,
    start: Option<f64>,
    end: Option<f64>,
    score: Option<f64>,
    coverage: Option<f64>,
    reason: Option<String>,
}
#[derive(Deserialize)]
struct Response {
    candidates: Vec<Candidate>,
}

pub fn validate_config(conf: &AlignmentConfig) -> Result<()> {
    if !conf.search_padding_seconds.is_finite()
        || !(0.0..=10.0).contains(&conf.search_padding_seconds)
        || !conf.max_shift_seconds.is_finite()
        || !(0.0..=10.0).contains(&conf.max_shift_seconds)
        || !conf.min_score.is_finite()
        || !(0.0..=1.0).contains(&conf.min_score)
        || !conf.min_coverage.is_finite()
        || !(0.0..=1.0).contains(&conf.min_coverage)
        || conf.timeout_seconds == 0
    {
        bail!("Invalid alignment settings: padding/shift must be 0–10 seconds, score/coverage 0–1, timeout positive");
    }
    Ok(())
}

fn python_path(conf: &AlignmentConfig) -> PathBuf {
    if let Some(path) = &conf.python {
        return path.clone();
    }
    if let Some(path) = std::env::var_os("SOKSAK_ALIGNMENT_PYTHON") {
        return path.into();
    }
    if let Some(home) = dirs::home_dir() {
        let python = home.join(".soksak/alignment-venv").join(if cfg!(windows) {
            "Scripts/python.exe"
        } else {
            "bin/python"
        });
        if python.is_file() {
            return python;
        }
    }
    PathBuf::from("python3")
}

fn run_aligner(
    audio: &ExtractedAudio,
    language: &str,
    segments: &[TranscriptSegment],
    conf: &AlignmentConfig,
) -> Result<Response> {
    let temp = TempDir::new()?;
    let script = temp.path().join("align.py");
    let request = temp.path().join("request.json");
    let output = temp.path().join("response.json");
    std::fs::write(&script, include_str!("align.py"))?;
    let duration = audio.duration_cs() as f64 / 100.0;
    let items: Vec<_> = segments.iter().enumerate().map(|(id, s)| {
        // Adjacent transcript midpoints prevent the aligner from grabbing the next
        // speaker's identical words. Original overlaps remain bounded by their own spans.
        let lower = if id > 0 { ((segments[id-1].end + s.start) as f64 / 200.0).min(s.start as f64 / 100.0) } else { 0.0 };
        let upper = if id+1 < segments.len() { ((s.end + segments[id+1].start) as f64 / 200.0).max(s.end as f64 / 100.0) } else { duration };
        serde_json::json!({"id": id, "text": s.text,
            "search_start": (s.start as f64 / 100.0 - conf.search_padding_seconds).max(lower).max(0.0),
            "search_end": (s.end as f64 / 100.0 + conf.search_padding_seconds).min(upper).min(duration)})
    }).collect();
    std::fs::write(
        &request,
        serde_json::to_vec(&serde_json::json!({
            "audio": audio.file.path(), "language": language, "model": conf.model, "segments": items
        }))?,
    )?;
    let mut stderr = NamedTempFile::new()?;
    let mut child = Command::new(python_path(conf))
        .arg(&script)
        .arg(&request)
        .arg(&output)
        .env("HF_HUB_DISABLE_TELEMETRY", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(stderr.reopen()?)
        .spawn()
        .context("Cannot start alignment Python; run scripts/setup_alignment.py")?;
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.into());
            }
        }
        if started.elapsed() >= Duration::from_secs(conf.timeout_seconds) {
            let _ = child.kill();
            let _ = child.wait();
            bail!("Alignment exceeded {} seconds", conf.timeout_seconds);
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    if !status.success() {
        let len = stderr.as_file().metadata()?.len();
        stderr.seek(SeekFrom::Start(len.saturating_sub(2000)))?;
        let mut tail = Vec::new();
        stderr.read_to_end(&mut tail)?;
        bail!(
            "Alignment runtime failed (run scripts/setup_alignment.py): {}",
            String::from_utf8_lossy(&tail).trim()
        );
    }
    serde_json::from_reader(std::fs::File::open(output)?).context("Invalid alignment response")
}

fn apply_candidates(
    segments: &mut [TranscriptSegment],
    candidates: Vec<Candidate>,
    duration_cs: i64,
    conf: &AlignmentConfig,
) -> Result<Vec<TimingChange>> {
    if candidates.len() != segments.len() {
        bail!("Alignment response changed segment count");
    }
    let mut by_id: Vec<Option<Candidate>> = (0..segments.len()).map(|_| None).collect();
    for candidate in candidates {
        let id = candidate.id;
        if id >= segments.len() || by_id[id].is_some() {
            bail!("Invalid or duplicate alignment segment ID");
        }
        by_id[id] = Some(candidate);
    }
    let original = segments.to_vec();
    let mut changes = Vec::new();
    for (id, (s, candidate)) in segments.iter_mut().zip(by_id).enumerate() {
        let c = candidate.context("Missing alignment segment")?;
        let mut status = c
            .reason
            .clone()
            .unwrap_or_else(|| "missing_alignment".into());
        if let (Some(start), Some(end), Some(score), Some(coverage)) =
            (c.start, c.end, c.score, c.coverage)
        {
            if c.reason.is_some() {
                // Explicit failure always wins over any accompanying numeric values.
            } else if [start, end, score, coverage].iter().all(|v| v.is_finite()) {
                let start_cs = (start * 100.0).round() as i64;
                let end_cs = (end * 100.0).round() as i64;
                status = if !(0.0..=1.0).contains(&score)
                    || !(0.0..=1.0).contains(&coverage)
                    || score < conf.min_score
                    || coverage < conf.min_coverage
                {
                    "low_confidence"
                } else if start < 0.0 || start_cs < 0 || end_cs > duration_cs || end_cs <= start_cs
                {
                    "invalid_range"
                } else if start_cs.abs_diff(s.start) as f64 > conf.max_shift_seconds * 100.0
                    || end_cs.abs_diff(s.end) as f64 > conf.max_shift_seconds * 100.0
                {
                    "excessive_shift"
                } else {
                    s.start = start_cs;
                    s.end = end_cs;
                    "aligned"
                }
                .into();
            } else {
                status = "non_finite_alignment".into();
            }
        }
        changes.push(TimingChange {
            id,
            original_start: original[id].start,
            original_end: original[id].end,
            start: s.start,
            end: s.end,
            status,
            score: c.score.filter(|v| v.is_finite()),
            coverage: c.coverage.filter(|v| v.is_finite()),
        });
    }
    // Reject newly introduced overlaps/order reversals, rather than truncating words.
    // Iterate because reverting one candidate can expose a conflict with its neighbor.
    loop {
        let mut revert = Vec::new();
        for i in 1..segments.len() {
            let new_overlap = segments[i - 1].end.saturating_sub(segments[i].start).max(0);
            let old_overlap = original[i - 1].end.saturating_sub(original[i].start).max(0);
            if new_overlap > old_overlap || segments[i].start < segments[i - 1].start {
                for j in [i - 1, i] {
                    if changes[j].status == "aligned" {
                        revert.push(j);
                    }
                }
            }
        }
        if revert.is_empty() {
            break;
        }
        for i in revert {
            segments[i].start = original[i].start;
            segments[i].end = original[i].end;
            changes[i].start = original[i].start;
            changes[i].end = original[i].end;
            changes[i].status = "neighbor_conflict".into();
        }
    }
    Ok(changes)
}

pub fn refine(
    audio: &ExtractedAudio,
    language: &str,
    mut segments: Vec<TranscriptSegment>,
    conf: &AlignmentConfig,
    pb: &impl Progress,
) -> Result<RefinedTranscript> {
    validate_config(conf)?;
    validate_timestamps(segments.iter().map(|s| (s.start, s.end)))?;
    if segments.iter().any(|s| s.end > audio.duration_cs()) {
        bail!("Transcript extends beyond decoded audio");
    }
    let mut report = TimingReport {
        version: 1,
        time_unit: "centiseconds",
        language: language.into(),
        audio_stream: audio.stream_index,
        timeline_origin_seconds: audio.timeline_origin_seconds,
        duration_cs: audio.duration_cs(),
        alignment_status: "off".into(),
        segments: Vec::new(),
    };
    if conf.mode != AlignmentMode::Off && !segments.is_empty() {
        pb.set_message("Aligning transcript to audio...");
        // Apply to a copy; malformed output must never leave partially modified times.
        let mut proposed = segments.clone();
        match run_aligner(audio, language, &segments, conf).and_then(|response| {
            apply_candidates(
                &mut proposed,
                response.candidates,
                audio.duration_cs(),
                conf,
            )
        }) {
            Ok(changes) => {
                report.segments = changes;
                report.alignment_status = "completed".into();
                segments = proposed;
            }
            Err(error) if conf.mode == AlignmentMode::Required => {
                return Err(error.context("Required alignment failed"))
            }
            Err(error) => {
                let message =
                    format!("Alignment unavailable; preserving original timing: {error:#}");
                eprintln!("{message}");
                log::warn!("{message}");
                pb.set_message(
                    "Alignment unavailable; original timing retained (see timing report)",
                );
                report.alignment_status = message;
            }
        }
    }
    if report.segments.is_empty() {
        report.segments = segments
            .iter()
            .enumerate()
            .map(|(id, s)| TimingChange {
                id,
                original_start: s.start,
                original_end: s.end,
                start: s.start,
                end: s.end,
                status: "original".into(),
                score: None,
                coverage: None,
            })
            .collect();
    }
    Ok(RefinedTranscript { segments, report })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn segment(start: i64, end: i64) -> TranscriptSegment {
        TranscriptSegment {
            start,
            end,
            text: " 원문 그대로! ".into(),
        }
    }
    fn candidate(id: usize, start: f64, end: f64) -> Candidate {
        Candidate {
            id,
            start: Some(start),
            end: Some(end),
            score: Some(0.95),
            coverage: Some(1.0),
            reason: None,
        }
    }
    #[test]
    fn alignment_changes_only_times_and_converts_seconds_once() {
        let mut s = vec![segment(100, 300)];
        let r = apply_candidates(
            &mut s,
            vec![candidate(0, 1.23, 2.78)],
            1000,
            &AlignmentConfig::default(),
        )
        .unwrap();
        assert_eq!((s[0].start, s[0].end), (123, 278));
        assert_eq!(s[0].text, " 원문 그대로! ");
        assert_eq!(r[0].original_start, 100);
    }
    #[test]
    fn rejects_bad_ranges_low_confidence_and_large_shifts() {
        for c in [
            candidate(0, f64::NAN, 2.0),
            candidate(0, 2.0, 1.0),
            candidate(0, 8.0, 9.0),
            Candidate {
                score: Some(0.1),
                ..candidate(0, 1.2, 2.8)
            },
            Candidate {
                coverage: Some(0.2),
                ..candidate(0, 1.2, 2.8)
            },
        ] {
            let mut s = vec![segment(100, 300)];
            let r = apply_candidates(&mut s, vec![c], 1000, &AlignmentConfig::default()).unwrap();
            assert_ne!(r[0].status, "aligned");
            assert_eq!((s[0].start, s[0].end), (100, 300));
        }
    }
    #[test]
    fn overlapping_candidates_revert_instead_of_cutting_off_words() {
        let mut s = vec![segment(100, 200), segment(210, 300)];
        let r = apply_candidates(
            &mut s,
            vec![candidate(0, 1.0, 2.3), candidate(1, 2.2, 3.0)],
            1000,
            &AlignmentConfig::default(),
        )
        .unwrap();
        assert!(r.iter().all(|r| r.status == "neighbor_conflict"));
        assert_eq!((s[0].end, s[1].start), (200, 210));
    }
    #[test]
    fn malformed_response_is_rejected_before_modifying_any_segment() {
        let mut s = vec![segment(100, 200), segment(300, 400)];
        assert!(apply_candidates(
            &mut s,
            vec![candidate(0, 1.2, 1.8), candidate(0, 3.2, 3.8)],
            1000,
            &AlignmentConfig::default()
        )
        .is_err());
        assert_eq!(s[0].start, 100);
    }
    #[test]
    fn neighboring_valid_candidate_is_rejected_when_other_candidate_falls_back() {
        let mut s = vec![segment(100, 200), segment(210, 300)];
        let rejected = Candidate {
            score: Some(0.1),
            ..candidate(1, 2.5, 3.0)
        };
        let r = apply_candidates(
            &mut s,
            vec![candidate(0, 1.1, 2.4), rejected],
            1000,
            &AlignmentConfig::default(),
        )
        .unwrap();
        assert_eq!(r[0].status, "neighbor_conflict");
        assert_eq!(s[0].end, 200);
    }
}
