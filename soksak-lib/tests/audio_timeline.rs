use soksak_lib::{ffmpeg_decoder, progress::Progress};
use std::{path::Path, process::Command};
use tempfile::TempDir;
struct Quiet;
impl Progress for Quiet {
    fn inc(&self, _: u64) {}
    fn set_position(&self, _: u64) {}
    fn set_message(&self, _: &str) {}
}
fn ffmpeg(args: &[&str], output: &Path) {
    let result = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(args)
        .arg(output)
        .output()
        .expect("FFmpeg is required for audio timeline tests");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
fn first_sound(samples: &[f32]) -> f64 {
    samples.iter().position(|v| v.abs() > 0.01).unwrap() as f64 / 16000.0
}
#[test]
fn delayed_audio_stays_delayed_relative_to_video() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("offset.mkv");
    ffmpeg(
        &[
            "-f",
            "lavfi",
            "-i",
            "color=s=160x90:r=10:d=4",
            "-itsoffset",
            "2",
            "-f",
            "lavfi",
            "-i",
            "sine=sample_rate=16000:duration=1",
            "-map",
            "0:v:0",
            "-map",
            "1:a:0",
            "-c:v",
            "ffv1",
            "-c:a",
            "pcm_s16le",
        ],
        &path,
    );
    let audio = ffmpeg_decoder::extract(&path, None, &Quiet).unwrap();
    assert!((first_sound(&audio.samples) - 2.0).abs() < 0.002);
    assert_eq!(audio.duration_cs(), 300);
}
#[test]
fn missing_packets_become_silence_instead_of_shortening_the_timeline() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("gap.mkv");
    ffmpeg(
        &[
            "-f",
            "lavfi",
            "-i",
            "sine=sample_rate=16000:duration=3",
            "-af",
            "aselect=not(between(t\\,1\\,2))",
            "-c:a",
            "pcm_s16le",
        ],
        &path,
    );
    let audio = ffmpeg_decoder::extract(&path, None, &Quiet).unwrap();
    assert_eq!(audio.duration_cs(), 300);
    assert!(audio.samples[20000..30000].iter().all(|v| v.abs() < 0.001));
    assert!(audio.samples[34000..35000].iter().any(|v| v.abs() > 0.01));
}
#[test]
fn nonzero_container_origin_is_normalized_once() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nonzero.mkv");
    ffmpeg(
        &[
            "-f",
            "lavfi",
            "-i",
            "sine=sample_rate=16000:duration=1",
            "-af",
            "asetpts=PTS+5/TB",
            "-c:a",
            "pcm_s16le",
        ],
        &path,
    );
    let audio = ffmpeg_decoder::extract(&path, None, &Quiet).unwrap();
    assert!((audio.timeline_origin_seconds - 5.0).abs() < 0.002);
    assert!(first_sound(&audio.samples) < 0.002);
    assert_eq!(audio.duration_cs(), 100);
}
#[test]
fn explicit_audio_stream_is_selected_and_invalid_index_is_rejected() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("tracks.mkv");
    ffmpeg(
        &[
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=16000:cl=mono:d=1",
            "-f",
            "lavfi",
            "-i",
            "sine=sample_rate=16000:duration=1",
            "-map",
            "0:a:0",
            "-map",
            "1:a:0",
            "-c:a",
            "pcm_s16le",
            "-disposition:a:0",
            "default",
            "-disposition:a:1",
            "0",
        ],
        &path,
    );
    let default = ffmpeg_decoder::extract(&path, None, &Quiet).unwrap();
    assert!(default.samples.iter().all(|v| *v == 0.0));
    let selected = ffmpeg_decoder::extract(&path, Some(1), &Quiet).unwrap();
    assert_eq!(selected.stream_index, 1);
    assert!(first_sound(&selected.samples) < 0.002);
    assert!(ffmpeg_decoder::extract(&path, Some(99), &Quiet).is_err());
}
