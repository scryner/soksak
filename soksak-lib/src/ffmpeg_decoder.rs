use crate::progress::Progress;
use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

pub const SAMPLE_RATE: usize = 16_000;

pub struct ExtractedAudio {
    pub file: NamedTempFile,
    pub samples: Vec<f32>,
    pub stream_index: u32,
    pub timeline_origin_seconds: f64,
}

impl ExtractedAudio {
    pub fn duration_cs(&self) -> i64 {
        (self.samples.len() * 100 / SAMPLE_RATE) as i64
    }
}

fn find_tool(name: &str) -> Result<PathBuf> {
    if let Ok(path) = which::which(name) {
        return Ok(path);
    }
    for dir in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"] {
        let path = Path::new(dir).join(name);
        if path.is_file() {
            return Ok(path);
        }
    }
    bail!("{name} not found. Please install FFmpeg (including ffprobe).")
}

#[derive(Deserialize)]
struct Probe {
    streams: Vec<Stream>,
    format: Format,
}
#[derive(Deserialize)]
struct Stream {
    index: u32,
    codec_type: String,
    start_time: Option<String>,
    #[serde(default)]
    disposition: Disposition,
}
#[derive(Deserialize, Default)]
struct Disposition {
    #[serde(default)]
    default: u32,
}
#[derive(Deserialize)]
struct Format {
    start_time: Option<String>,
    duration: Option<String>,
}

fn finite_time(value: Option<&str>) -> Option<f64> {
    value?.parse::<f64>().ok().filter(|x| x.is_finite())
}

impl Probe {
    fn audio_stream(&self, requested: Option<u32>) -> Result<u32> {
        let audio: Vec<_> = self
            .streams
            .iter()
            .filter(|s| s.codec_type == "audio")
            .collect();
        if let Some(index) = requested {
            return audio
                .iter()
                .find(|s| s.index == index)
                .map(|s| s.index)
                .ok_or_else(|| anyhow!("Input stream {index} is not an audio stream"));
        }
        audio
            .iter()
            .find(|s| s.disposition.default != 0)
            .or_else(|| audio.first())
            .map(|s| s.index)
            .ok_or_else(|| anyhow!("Input has no audio stream"))
    }

    fn origin(&self) -> f64 {
        finite_time(self.format.start_time.as_deref()).unwrap_or_else(|| {
            self.streams
                .iter()
                .filter(|s| matches!(s.codec_type.as_str(), "audio" | "video"))
                .filter_map(|s| finite_time(s.start_time.as_deref()))
                .min_by(f64::total_cmp)
                .unwrap_or(0.0)
        })
    }
}

/// Decode on the container's playback timeline, not the first audio packet's timeline.
/// PTS gaps become PCM silence; all downstream engines share these exact samples.
pub fn extract<P: AsRef<Path>>(
    input: P,
    stream: Option<u32>,
    pb: &impl Progress,
) -> Result<ExtractedAudio> {
    let input = input.as_ref();
    let probe_output = Command::new(find_tool("ffprobe")?)
        .args([
            "-v",
            "error",
            "-show_streams",
            "-show_format",
            "-of",
            "json",
        ])
        .arg(input)
        .stdin(Stdio::null())
        .output()
        .context("Failed to run ffprobe")?;
    if !probe_output.status.success() {
        bail!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&probe_output.stderr)
        );
    }
    let probe: Probe =
        serde_json::from_slice(&probe_output.stdout).context("Invalid ffprobe output")?;
    let stream_index = probe.audio_stream(stream)?;
    let origin = probe.origin();
    let duration = finite_time(probe.format.duration.as_deref()).filter(|d| *d > 0.0);
    let file = NamedTempFile::with_suffix(".wav")?;
    let filter =
        format!("asetpts=PTS-({origin:.9})/TB,aresample={SAMPLE_RATE}:async=1:first_pts=0");
    let mut child = Command::new(find_tool("ffmpeg")?)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-y",
            "-copyts",
            "-i",
        ])
        .arg(input)
        .args([
            "-map",
            &format!("0:{stream_index}"),
            "-vn",
            "-af",
            &filter,
            "-ac",
            "1",
            "-c:a",
            "pcm_s16le",
            "-progress",
            "pipe:2",
        ])
        .arg(file.path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to run ffmpeg")?;
    let mut errors = std::collections::VecDeque::new();
    let stderr = child.stderr.take().context("Missing ffmpeg stderr")?;
    for line in BufReader::new(stderr).lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.into());
            }
        };
        if let (Some(value), Some(total)) = (line.strip_prefix("out_time_us="), duration) {
            if let Ok(us) = value.parse::<f64>() {
                pb.set_position((100.0 * us / 1_000_000.0 / total).clamp(0.0, 100.0) as u64);
            }
        } else if !line.contains('=') {
            if errors.len() == 16 {
                errors.pop_front();
            }
            errors.push_back(line);
        }
    }
    if !child.wait()?.success() {
        bail!(
            "FFmpeg audio extraction failed: {}",
            errors.into_iter().collect::<Vec<_>>().join("\n")
        );
    }
    let mut reader = hound::WavReader::open(file.path())?;
    let spec = reader.spec();
    if spec.sample_rate != SAMPLE_RATE as u32 || spec.channels != 1 || spec.bits_per_sample != 16 {
        bail!("Unexpected decoded PCM format");
    }
    let samples = reader
        .samples::<i16>()
        .map(|s| s.map(|s| s as f32 / 32768.0))
        .collect::<Result<Vec<_>, _>>()?;
    if samples.is_empty() {
        bail!("Input contains no audio samples");
    }
    Ok(ExtractedAudio {
        file,
        samples,
        stream_index,
        timeline_origin_seconds: origin,
    })
}

pub fn read_file<P: AsRef<Path>>(input: P, pb: &impl Progress) -> Result<Vec<f32>> {
    Ok(extract(input, None, pb)?.samples)
}

pub fn file<P: AsRef<Path>>(input: P, pb: &impl Progress) -> Result<NamedTempFile> {
    Ok(extract(input, None, pb)?.file)
}
