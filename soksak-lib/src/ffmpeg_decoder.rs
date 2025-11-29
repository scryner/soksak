use anyhow::{Result, anyhow};
use audrey::Reader;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use tempfile::NamedTempFile;

// ffmpeg -i input.mp3 -ar 16000 output.wav
fn use_ffmpeg<P: AsRef<Path>>(input_path: P) -> Result<NamedTempFile> {
    println!("Using ffmpeg to convert audio file");

    let temp_file = NamedTempFile::with_suffix(".wav")?;
    let temp_path = temp_file.path();

    let mut pid = Command::new("ffmpeg")
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
            "error",
        ])
        .stdin(Stdio::null())
        .spawn()?;

    if pid.wait()?.success() {
        println!("Audio file converted successfully");
        Ok(temp_file)
    } else {
        Err(anyhow!("unable to convert file"))
    }
}

pub fn read_file<P: AsRef<Path>>(audio_file_path: P) -> Result<Vec<f32>> {
    let temp_file = use_ffmpeg(&audio_file_path)?;

    let mut reader = Reader::new(temp_file.reopen()?)?;
    let audio_buf: Vec<i16> = reader.samples().collect::<Result<_, _>>()?;
    let mut output = vec![0.0f32; audio_buf.len()];

    whisper_rs::convert_integer_to_float_audio(&audio_buf, &mut output)?;
    Ok(output)
    // temp_file is automatically deleted when it goes out of scope here
}

#[allow(dead_code)]
pub fn file<P: AsRef<Path>>(audio_file_path: P) -> Result<NamedTempFile> {
    let temp_file = use_ffmpeg(&audio_file_path)?;
    Ok(temp_file)
}
