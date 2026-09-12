use soksak_lib::{
    config::{AlignmentConfig, AlignmentDevice, AlignmentMode, WhisperConfig},
    ffmpeg_decoder::ExtractedAudio,
    progress::Progress,
    transcribe::{alignment, validate_timestamps, TranscriptSegment},
};
use tempfile::NamedTempFile;
struct Quiet;
impl Progress for Quiet {
    fn inc(&self, _: u64) {}
    fn set_position(&self, _: u64) {}
    fn set_message(&self, _: &str) {}
}
fn audio() -> ExtractedAudio {
    ExtractedAudio {
        file: NamedTempFile::new().unwrap(),
        samples: vec![0.0; 16000 * 5],
        stream_index: 0,
        timeline_origin_seconds: 0.0,
    }
}
fn segment() -> TranscriptSegment {
    TranscriptSegment {
        start: 100,
        end: 200,
        text: "Keep the exact original.".into(),
    }
}
#[test]
fn missing_runtime_falls_back_in_auto_and_fails_in_required() {
    let audio = audio();
    let mut conf = AlignmentConfig {
        python: Some(audio.file.path().join("missing-python")),
        ..Default::default()
    };
    let result = alignment::refine(&audio, "en", vec![segment()], &conf, &Quiet).unwrap();
    assert_eq!(result.segments[0].start, 100);
    assert_eq!(result.segments[0].text, segment().text);
    assert!(result.report.alignment_status.contains("unavailable"));
    conf.mode = AlignmentMode::Required;
    assert!(alignment::refine(&audio, "en", vec![segment()], &conf, &Quiet).is_err());
}
#[test]
fn off_does_not_launch_runtime_and_invalid_input_is_rejected() {
    let audio = audio();
    let conf = AlignmentConfig {
        mode: AlignmentMode::Off,
        python: Some(audio.file.path().join("missing-python")),
        ..Default::default()
    };
    assert_eq!(
        alignment::refine(&audio, "en", vec![segment()], &conf, &Quiet)
            .unwrap()
            .report
            .alignment_status,
        "off"
    );
    let bad = TranscriptSegment {
        end: 600,
        ..segment()
    };
    assert!(alignment::refine(&audio, "en", vec![bad], &conf, &Quiet).is_err());
}
#[test]
fn timestamp_validation_rejects_invalid_and_reversed_ranges_but_allows_overlap() {
    for ranges in [
        vec![(-1, 100)],
        vec![(100, 100)],
        vec![(100, 90)],
        vec![(100, 300), (99, 200)],
    ] {
        assert!(validate_timestamps(ranges).is_err());
    }
    assert!(validate_timestamps([(100, 300), (200, 400)]).is_ok());
}
#[test]
fn existing_yaml_profiles_keep_working_and_partial_alignment_uses_defaults() {
    let old: WhisperConfig = serde_yaml::from_str("vad: true\nbeam_size: 5\n").unwrap();
    assert_eq!(old.alignment.mode, AlignmentMode::Auto);
    assert_eq!(old.alignment.device, AlignmentDevice::Auto);
    assert!(old.alignment.preserve_original_end);
    assert_eq!(old.alignment.end_padding_seconds, 0.2);
    let partial: WhisperConfig = serde_yaml::from_str("alignment:\n  mode: required\n").unwrap();
    assert_eq!(partial.alignment.mode, AlignmentMode::Required);
    assert_eq!(partial.alignment.min_coverage, 0.8);
}

#[test]
#[cfg(unix)]
fn hung_runtime_is_terminated_and_original_timing_survives() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::TempDir::new().unwrap();
    let runner = dir.path().join("hung-runtime");
    // exec replaces the shell, so the process under test owns no detached children.
    std::fs::write(&runner, "#!/bin/sh\nexec /bin/sleep 30\n").unwrap();
    std::fs::set_permissions(&runner, std::fs::Permissions::from_mode(0o700)).unwrap();
    let conf = AlignmentConfig {
        python: Some(runner),
        timeout_seconds: 1,
        ..Default::default()
    };
    let started = std::time::Instant::now();
    let result = alignment::refine(&audio(), "en", vec![segment()], &conf, &Quiet).unwrap();
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
    assert!(result
        .report
        .alignment_status
        .contains("exceeded 1 seconds"));
    assert_eq!(result.segments[0].start, 100);
}
