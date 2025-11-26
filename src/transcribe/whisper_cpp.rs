use std::{
    ffi::{c_int, c_void},
    path::Path,
};

use anyhow::{Result, anyhow};

use std::io::Write;
use tempfile::NamedTempFile;
use whisper_rs::{FullParams, WhisperContext, WhisperContextParameters, WhisperVadParams};

use crate::{
    config::{Language, TranscriptionConfig, WhisperConfig},
    ffmpeg_decoder,
    transcribe::TranscriptSegment,
};

unsafe extern "C" fn whisper_progress_callback(
    _ctx: *mut c_void,
    _state: *mut c_void,
    progress: c_int,
    user_data: *mut c_void,
) {
    if !user_data.is_null() {
        unsafe {
            let pb = &*(user_data as *mut indicatif::ProgressBar);
            pb.set_position(progress as u64);
        }
    }
}

pub struct Whisper {
    ctx: WhisperContext,
    lang: Language,
}

const DEFAULT_BEAM_SIZE: u32 = 5;
const DEFAULT_PATIENCE: f32 = 1.0;

impl Whisper {
    pub fn new(conf: &TranscriptionConfig, lang: Language) -> Result<Self> {
        // get model path according to lang
        let model_config = conf
            .models
            .get(&lang)
            .ok_or_else(|| anyhow!("Model not configured for language: {:?}", lang))?;

        // make whisper context
        let param = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(&model_config.model, param)?;

        Ok(Self { ctx, lang })
    }

    pub fn transcribe<P: AsRef<Path>>(
        &mut self,
        audio: P,
        conf: &WhisperConfig,
        pb: &mut indicatif::ProgressBar,
    ) -> Result<Vec<TranscriptSegment>> {
        // make parameters
        let mut params = FullParams::new(whisper_rs::SamplingStrategy::BeamSearch {
            beam_size: conf.beam_size.unwrap_or(DEFAULT_BEAM_SIZE) as c_int,
            patience: conf.patience.unwrap_or(DEFAULT_PATIENCE),
        });

        // Silero VAD model (MIT License)
        // Copyright (c) 2021 Silero Team
        const SILERO_MODEL: &[u8] = include_bytes!("models/silero_vad.bin");

        // Keep the temp file alive as long as params is used
        let _vad_temp_file;

        if conf.vad.unwrap_or(true) {
            let mut vod_params = WhisperVadParams::new();
            vod_params.set_min_speech_duration(150);
            vod_params.set_min_silence_duration(200);
            vod_params.set_speech_pad(30);
            params.set_no_context(true);

            params.set_vad_params(vod_params);

            // Write embedded model to a temporary file using tempfile crate
            let mut temp_file = NamedTempFile::new()?;
            temp_file.write_all(SILERO_MODEL)?;

            params.set_vad_model_path(Some(
                temp_file.path().to_str().ok_or(anyhow!("Invalid path"))?,
            ));
            params.enable_vad(true);

            _vad_temp_file = Some(temp_file);
        } else {
            _vad_temp_file = None;
        }

        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_token_timestamps(false);

        params.set_temperature(conf.temperature.unwrap_or(0.0));

        params.set_language(Some(self.lang.as_str()));
        match conf.initial_prompt.as_ref() {
            Some(prompt) => params.set_initial_prompt(prompt),
            None => {}
        }

        // Set the progress callback to update the provided ProgressBar
        unsafe {
            params.set_progress_callback(Some(std::mem::transmute(
                whisper_progress_callback as *const (),
            )));
            params.set_progress_callback_user_data(pb as *mut _ as *mut c_void);
        }

        let audio = ffmpeg_decoder::read_file(audio)?;

        let mut state = self.ctx.create_state()?;
        state.full(params, &audio)?;

        let num_segments = state.full_n_segments();
        if num_segments < 1 {
            return Err(anyhow!("no segments found"));
        }

        let mut words = Vec::with_capacity(num_segments as usize);

        for segment in state.as_iter() {
            let text = segment.to_str_lossy()?.to_string();
            let start = segment.start_timestamp();
            let end = segment.end_timestamp();

            words.push(TranscriptSegment { start, end, text });
        }

        Ok(words)
    }
}

// use anyhow::{Context, Result};
// use std::path::Path;
// use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
// use std::sync::Arc;
// use std::sync::Mutex;

// #[derive(Debug, Clone, serde::Serialize)]
// pub struct TranscriptSegment {
//     pub start: i64, // milliseconds
//     pub end: i64,   // milliseconds
//     pub text: String,
// }

// pub fn extract_audio(input_path: &Path) -> Result<Vec<f32>> {
//     // Use ffmpeg to extract audio to 16kHz mono WAV
//     // For simplicity in this step, we'll use std::process::Command to run ffmpeg and save to a temp file, then read it.
//     // A better approach would be piping stdout, but reading a file is safer for now.

//     let temp_dir = std::env::temp_dir();
//     let temp_wav = temp_dir.join(format!("soksak_{}.wav", uuid::Uuid::new_v4()));

//     let status = std::process::Command::new("ffmpeg")
//         .arg("-i")
//         .arg(input_path)
//         .arg("-ar")
//         .arg("16000")
//         .arg("-ac")
//         .arg("1")
//         .arg("-c:a")
//         .arg("pcm_s16le")
//         .arg(&temp_wav)
//         .arg("-y")
//         .arg("-v")
//         .arg("quiet")
//         .status()
//         .context("Failed to run ffmpeg")?;

//     if !status.success() {
//         anyhow::bail!("ffmpeg failed");
//     }

//     let mut reader = hound::WavReader::open(&temp_wav).context("Failed to open wav file")?;
//     let samples: Vec<f32> = reader
//         .samples::<i16>()
//         .map(|s| s.unwrap() as f32 / 32768.0)
//         .collect();

//     // Cleanup
//     let _ = std::fs::remove_file(temp_wav);

//     Ok(samples)
// }

// pub fn transcribe(
//     model_path: &str,
//     audio_data: &[f32],
//     lang: &str,
//     vad: bool,
//     initial_prompt: Option<&str>,
//     progress_callback: impl Fn(i32) + Send + 'static,
// ) -> Result<Vec<TranscriptSegment>> {
//     let ctx = WhisperContext::new_with_params(
//         model_path,
//         WhisperContextParameters::default(),
//     )
//     .context("Failed to load model")?;

//     let mut state = ctx.create_state().context("Failed to create state")?;

//     let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

//     params.set_language(Some(lang));
//     params.set_print_special(false);
//     params.set_print_progress(false);
//     params.set_print_realtime(false);
//     params.set_print_timestamps(false);

//     if let Some(prompt) = initial_prompt {
//         params.set_initial_prompt(prompt);
//     }

//     // VAD is not directly exposed in simple FullParams in older whisper-rs versions easily without custom logic,
//     // but let's check if we can just ignore it for now or if it's crucial.
//     // The user asked for it, but standard whisper.cpp handles VAD internally if configured?
//     // Actually whisper.cpp has some VAD support but it might be complex to toggle via params.
//     // We will proceed without explicit VAD param for now unless we find it in FullParams.
//     // (Checking docs: FullParams doesn't seem to have a simple 'vad' boolean in standard bindings, usually it's about token suppression)

//     // Progress callback
//     // whisper-rs allows setting a progress callback.
//     // params.set_progress_callback(...)
//     // But it requires unsafe or careful handling.
//     // For now, we will simulate progress or just run it.
//     // Actually, let's try to use the segment callback to update progress.

//     let audio_len_ms = (audio_data.len() as f64 / 16000.0 * 1000.0) as i64;
//     let progress_callback = Arc::new(Mutex::new(progress_callback));

//     params.set_new_segment_callback(Some(Box::new(move |state, _n_new| {
//         let num_segments = state.full_n_segments().unwrap_or(0);
//         if num_segments > 0 {
//             let last_segment_end = state.full_get_segment_t1(num_segments - 1).unwrap_or(0);
//             // t1 is in centiseconds (10ms) usually in whisper.cpp, let's verify.
//             // whisper.cpp: t0, t1 are int64_t, usually 10ms units.
//             let current_ms = last_segment_end * 10;
//             let percentage = (current_ms as f64 / audio_len_ms as f64 * 100.0) as i32;
//             let cb = progress_callback.lock().unwrap();
//             cb(percentage.min(100));
//         }
//     })));

//     state.full(params, audio_data).context("failed to run model")?;

//     let num_segments = state.full_n_segments().context("failed to get segments")?;
//     let mut segments = Vec::new();

//     for i in 0..num_segments {
//         let start = state.full_get_segment_t0(i).unwrap_or(0) * 10; // to ms
//         let end = state.full_get_segment_t1(i).unwrap_or(0) * 10;   // to ms
//         let text = state.full_get_segment_text(i).unwrap_or_default();

//         segments.push(TranscriptSegment {
//             start,
//             end,
//             text: text.to_string(),
//         });
//     }

//     Ok(segments)
// }
