//! Opt-in integration test: requires a local Whisper model and the alignment runtime.
use soksak_lib::{
    config::{
        AlignmentMode, Language, TranscriptionConfig, TranscriptionEngine,
        TranscriptionModelConfig, WhisperConfig,
    },
    ffmpeg_decoder::{self, SAMPLE_RATE},
    progress::Progress,
    transcribe::{whisper_cpp::Whisper, TranscriptSegment},
};
use std::{collections::HashMap, path::Path};
use tempfile::TempDir;

struct Quiet;
impl Progress for Quiet {
    fn inc(&self, _: u64) {}
    fn set_position(&self, _: u64) {}
    fn set_message(&self, _: &str) {}
}

#[tokio::test]
#[ignore = "requires SOKSAK_TEST_MODEL and scripts/setup_alignment.py"]
async fn real_whisper_and_alignment_preserve_text_and_long_silence_offsets() {
    check_sync(true).await;
}

#[tokio::test]
#[ignore = "requires SOKSAK_TEST_MODEL and scripts/setup_alignment.py"]
async fn real_whisper_without_vad_records_alignment_or_fallback_without_rewriting() {
    check_sync(false).await;
}

async fn check_sync(vad: bool) {
    let model =
        std::env::var("SOKSAK_TEST_MODEL").expect("Set an absolute local Whisper model path");
    assert!(Path::new(&model).is_file());
    let conf = TranscriptionConfig {
        models: HashMap::from([(
            Language::English,
            TranscriptionModelConfig {
                engine: TranscriptionEngine::WhisperCpp,
                model,
            },
        )]),
    };
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/transcribe/whisperkit/harvard.wav");
    let audio = ffmpeg_decoder::extract(source, None, &Quiet).unwrap();
    let excerpt = &audio.samples[..SAMPLE_RATE * 8];
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("repeated.wav");
    let mut pcm = vec![0.0; SAMPLE_RATE * 5];
    pcm.extend_from_slice(excerpt);
    pcm.extend(vec![0.0; SAMPLE_RATE * 30]);
    pcm.extend_from_slice(excerpt);
    pcm.extend(vec![0.0; SAMPLE_RATE]);
    let mut wav = hound::WavWriter::create(
        &input,
        hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for sample in pcm {
        wav.write_sample((sample * 32768.0) as i16).unwrap();
    }
    wav.finalize().unwrap();

    let mut whisper = Whisper::new(&conf, Language::English).await.unwrap();
    let mut settings = WhisperConfig::default();
    settings.vad = Some(vad);
    settings.alignment.mode = AlignmentMode::Required;
    let aligned = whisper
        .transcribe(&input, &settings, &Quiet, &Quiet)
        .unwrap();
    let raw: Vec<TranscriptSegment> = serde_json::from_reader(
        std::fs::File::open(dir.path().join("repeated.raw.transcript.json")).unwrap(),
    )
    .unwrap();
    assert!(!aligned.is_empty());
    assert_eq!(aligned.len(), raw.len());
    for (after, before) in aligned.iter().zip(&raw) {
        assert_eq!(
            after.text, before.text,
            "Alignment must not rewrite transcript text"
        );
        assert!(after.start >= 0 && after.end <= 5200 && after.start < after.end);
        assert!(
            after.end >= before.end,
            "Default alignment must not shorten subtitle display"
        );
        if vad {
            assert!(after.start >= 490);
            assert!(
                after.end <= 1350 || after.start >= 4250,
                "Caption fell into the inserted silence: {after:?}"
            );
        }
    }
    let report: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(dir.path().join("repeated.timing.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report["alignment_status"], "completed");
    assert!(report["alignment_seconds"].as_f64().unwrap() > 0.0);
    assert!(matches!(
        report["runtime"]["device"].as_str(),
        Some("cpu" | "mps" | "cuda")
    ));
    eprintln!("Alignment runtime: {}", report["runtime"]);
    if !vad {
        // Control run: Whisper can anchor captions at 0/30 seconds during silence.
        // Large alignment corrections must fall back rather than bypass max_shift.
        for ((change, after), before) in report["segments"]
            .as_array()
            .unwrap()
            .iter()
            .zip(&aligned)
            .zip(&raw)
        {
            if change["status"] == "aligned" {
                assert!(after.start.abs_diff(before.start) <= 200);
                assert!(after.end.abs_diff(before.end) <= 200);
            } else {
                assert_eq!((after.start, after.end), (before.start, before.end));
            }
        }
        eprintln!(
            "VAD-off control raw: {raw:?}; timing decisions: {}",
            report["segments"]
        );
        return;
    }
    assert!(
        report["segments"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["status"] == "aligned"),
        "No alignment was accepted: {report}"
    );
    let first: Vec<_> = aligned.iter().filter(|s| s.end < 1350).collect();
    let second: Vec<_> = aligned.iter().filter(|s| s.start >= 4250).collect();
    assert!(!first.is_empty() && !second.is_empty());
    // Identical audio appears exactly 38 seconds later, including the 30-second gap.
    assert!(
        (second[0].start - first[0].start - 3800).abs() <= 15,
        "Repeated speech drifted: {first:?} vs {second:?}"
    );
    eprintln!(
        "Real model sync check (vad={vad}): {} captions, first starts {}cs / {}cs; statuses: {}",
        aligned.len(),
        first[0].start,
        second[0].start,
        report["segments"]
    );
}
