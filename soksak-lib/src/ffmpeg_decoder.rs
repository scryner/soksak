use crate::progress::Progress;
use anyhow::{anyhow, Result};
use audrey::Reader;
use regex::Regex;
use std::io::{BufReader, Read};
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use tempfile::NamedTempFile;

// Helper to find ffmpeg executable
fn find_ffmpeg() -> Result<std::path::PathBuf> {
    // 1. Try "ffmpeg" from PATH
    if let Ok(path) = which::which("ffmpeg") {
        return Ok(path);
    }

    // 2. Try common macOS paths
    let common_paths = [
        "/opt/homebrew/bin/ffmpeg", // Apple Silicon Homebrew
        "/usr/local/bin/ffmpeg",    // Intel Homebrew / Standard
        "/usr/bin/ffmpeg",          // System (rare on modern macOS)
    ];

    for path in common_paths {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }

    Err(anyhow!("ffmpeg not found. Please install ffmpeg."))
}

// ffmpeg -i input.mp3 -ar 16000 output.wav
fn use_ffmpeg<P: AsRef<Path>>(input_path: P, pb: &impl Progress) -> Result<NamedTempFile> {
    // println!("Using ffmpeg to convert audio file");

    let temp_file = NamedTempFile::with_suffix(".wav")?;
    let temp_path = temp_file.path();

    let ffmpeg_path = find_ffmpeg()?;

    let mut child = Command::new(ffmpeg_path)
        .args([
            "-i",
            input_path
                .as_ref()
                .to_str()
                .ok_or_else(|| anyhow!("invalid path"))?,
            "-ar",
            "16000",
            "-ac",
            "1",
            "-c:a",
            "pcm_s16le",
            temp_path.to_str().unwrap(),
            "-hide_banner",
            "-y",
            "-loglevel",
            "info", // Need info to see progress
            "-progress",
            "pipe:2",
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    let stderr = child.stderr.take().ok_or_else(|| anyhow!("no stderr"))?;
    let mut reader = BufReader::new(stderr);

    let re_duration = Regex::new(r"Duration: (\d{2}):(\d{2}):(\d{2})\.(\d{2})").unwrap();
    // Key-value parsing does not strictly require regex, but we will search for the specific line

    let mut total_duration_secs: Option<f64> = None;
    let mut buffer = Vec::new();
    let mut byte_buf = [0u8; 1];

    // Read byte by byte to handle both \r and \n properly
    loop {
        match reader.read(&mut byte_buf) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let b = byte_buf[0];
                if b == b'\r' || b == b'\n' {
                    if !buffer.is_empty() {
                        let line = String::from_utf8_lossy(&buffer);

                        // Parse Total Duration
                        if total_duration_secs.is_none() {
                            if let Some(caps) = re_duration.captures(&line) {
                                if let (Ok(hours), Ok(mins), Ok(secs), Ok(centis)) = (
                                    caps[1].parse::<f64>(),
                                    caps[2].parse::<f64>(),
                                    caps[3].parse::<f64>(),
                                    caps[4].parse::<f64>(),
                                ) {
                                    total_duration_secs =
                                        Some(hours * 3600.0 + mins * 60.0 + secs + centis / 100.0);
                                }
                            }
                        }

                        // Parse out_time_us for progress
                        if let Some(total) = total_duration_secs {
                            if line.starts_with("out_time_us=") {
                                let val_str = &line["out_time_us=".len()..];
                                if let Ok(us) = val_str.trim().parse::<i64>() {
                                    let current_secs = us as f64 / 1_000_000.0;
                                    if current_secs >= 0.0 {
                                        let progress = (current_secs / total * 100.0) as u64;
                                        pb.set_position(progress.min(100));
                                    }
                                }
                            }
                        }

                        buffer.clear();
                    }
                } else {
                    buffer.push(b);
                }
            }
            Err(_) => break, // Error
        }
    }

    if child.wait()?.success() {
        // println!("Audio file converted successfully");
        // Don't finish the progress bar here. The caller should do it.
        // soksak-lib/src/transcribe/whisper_cpp/mod.rs does call finish_with_message("Audio extracted")
        Ok(temp_file)
    } else {
        Err(anyhow!("unable to convert file"))
    }
}

pub fn read_file<P: AsRef<Path>>(audio_file_path: P, pb: &impl Progress) -> Result<Vec<f32>> {
    let temp_file = use_ffmpeg(&audio_file_path, pb)?;

    let mut reader = Reader::new(temp_file.reopen()?)?;
    let audio_buf: Vec<i16> = reader.samples().collect::<Result<_, _>>()?;
    let mut output = vec![0.0f32; audio_buf.len()];

    whisper_rs::convert_integer_to_float_audio(&audio_buf, &mut output)?;
    Ok(output)
    // temp_file is automatically deleted when it goes out of scope here
}

#[allow(dead_code)]
pub fn file<P: AsRef<Path>>(audio_file_path: P, pb: &impl Progress) -> Result<NamedTempFile> {
    let temp_file = use_ffmpeg(&audio_file_path, pb)?;
    Ok(temp_file)
}
