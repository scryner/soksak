use crate::progress::Progress;
use anyhow::{anyhow, Result};
use audrey::Reader;
use regex::Regex;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use tempfile::NamedTempFile;

// ffmpeg -i input.mp3 -ar 16000 output.wav
fn use_ffmpeg<P: AsRef<Path>>(input_path: P, pb: &impl Progress) -> Result<NamedTempFile> {
    // println!("Using ffmpeg to convert audio file");

    let temp_file = NamedTempFile::with_suffix(".wav")?;
    let temp_path = temp_file.path();

    let mut child = Command::new("ffmpeg")
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
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    let stderr = child.stderr.take().ok_or_else(|| anyhow!("no stderr"))?;
    let mut reader = BufReader::new(stderr);

    let re_duration = Regex::new(r"Duration: (\d{2}):(\d{2}):(\d{2})\.(\d{2})").unwrap();
    let re_time = Regex::new(r"time=(\d{2}):(\d{2}):(\d{2})\.(\d{2})").unwrap();

    let mut total_duration_secs: Option<f64> = None;
    let mut buffer = Vec::new();

    // Read byte by byte or chunks to handle \r
    // Using read_until would be easier but we need to handle both \n and \r
    // Actually, ffmpeg uses \r for status lines.

    // We can loop and separate by \r or \n
    loop {
        buffer.clear();
        let bytes_read = reader.read_until(b'\r', &mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        let line = String::from_utf8_lossy(&buffer);

        // Also check if there are internal newlines if read_until skipped them?
        // read_until stops AT delimiter.
        // If we have mixed \n and \r, it might be tricky.
        // ffmpeg usually does:
        // Duration: ... \n
        // ...
        // time=... \r time=... \r

        // If we read until \r, we might miss \n lines if they don't have \r?
        // But read_until(b'\r') will read everything up to \r.
        // If there is a \n in between, it will be in the buffer. we should scan the buffer.

        // However, standard ffmpeg output lines end with \n.
        // Progress lines end with \r.
        // If I use read_until(b'\r'), and a line ends with \n but NOT \r, it will keep reading until next \r.
        // That effectively buffers standard lines until the first progress line appears.
        // That is acceptable since we only care about "Duration" (early) and "time=" (progress).
        // "Duration" comes early.

        // Let's rely on the fact that we can parse the string in the buffer.
        // Regex search in the buffer.

        if let Some(caps) = re_duration.captures(&line) {
            let hours: f64 = caps[1].parse()?;
            let mins: f64 = caps[2].parse()?;
            let secs: f64 = caps[3].parse()?;
            let centis: f64 = caps[4].parse()?;
            total_duration_secs = Some(hours * 3600.0 + mins * 60.0 + secs + centis / 100.0);
        }

        if let Some(caps) = re_time.captures(&line) {
            if let Some(total) = total_duration_secs {
                let hours: f64 = caps[1].parse()?;
                let mins: f64 = caps[2].parse()?;
                let secs: f64 = caps[3].parse()?;
                let centis: f64 = caps[4].parse()?;
                let current_secs = hours * 3600.0 + mins * 60.0 + secs + centis / 100.0;

                let progress = (current_secs / total * 100.0) as u64;
                pb.set_position(progress.min(100));
            }
        }
    }

    if child.wait()?.success() {
        // println!("Audio file converted successfully");
        pb.finish();
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
