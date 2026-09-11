use crate::{
    config::{Language, TranscriptionConfig, WhisperConfig},
    ffmpeg_decoder,
    progress::Progress,
    transcribe::{
        alignment,
        timing::{speech_windows, AudioWindow},
        TranscriptSegment,
    },
};
use anyhow::{anyhow, bail, Context, Result};
use std::{ffi::c_void, io::Write, path::Path};
use tempfile::NamedTempFile;
use whisper_rs::{
    FullParams, WhisperContext, WhisperContextParameters, WhisperSysContext, WhisperSysState,
    WhisperVadContext, WhisperVadContextParams, WhisperVadParams,
};

struct WindowProgress<'a> {
    progress: &'a dyn Progress,
    completed: usize,
    window_samples: usize,
    total_samples: usize,
}

unsafe extern "C" fn report_progress(
    _context: *mut WhisperSysContext,
    _state: *mut WhisperSysState,
    percent: i32,
    user_data: *mut c_void,
) {
    // The borrowed data lives through the synchronous state.full() call below.
    let Some(data) = (unsafe { (user_data as *const WindowProgress<'_>).as_ref() }) else {
        return;
    };
    let completed = data.completed + data.window_samples * percent.clamp(0, 100) as usize / 100;
    // A user-provided progress handler must not unwind across the native callback boundary.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        data.progress
            .set_position((100 * completed / data.total_samples) as u64);
    }));
}

pub struct Whisper {
    ctx: WhisperContext,
    lang: Language,
}

impl Whisper {
    pub async fn new(conf: &TranscriptionConfig, lang: Language) -> Result<Self> {
        whisper_rs::install_logging_hooks();
        let model_path = conf
            .models
            .get(&lang)
            .ok_or_else(|| anyhow!("Model not configured: {lang:?}"))?
            .resolve_model_path()
            .await?;
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().context("Invalid model path")?,
            WhisperContextParameters::default(),
        )?;
        Ok(Self { ctx, lang })
    }

    pub fn transcribe<P: AsRef<Path>>(
        &mut self,
        input: P,
        conf: &WhisperConfig,
        pb_extract: &impl Progress,
        pb: &impl Progress,
    ) -> Result<Vec<TranscriptSegment>> {
        alignment::validate_config(&conf.alignment)?;
        let input = input.as_ref();
        pb_extract.set_message("Extracting audio on the video timeline...");
        let audio = ffmpeg_decoder::extract(input, conf.audio_stream, pb_extract)?;
        pb_extract.finish_with_message("Audio extracted");
        pb.set_length(100);
        pb.set_position(0);
        let windows = if conf.vad.unwrap_or(true) {
            pb.set_message("Detecting speech...");
            detect_windows(&audio.samples)?
        } else {
            vec![AudioWindow {
                start: 0,
                end: audio.samples.len(),
            }]
        };
        if windows.is_empty() {
            bail!("No speech detected");
        }
        let total_samples: usize = windows.iter().map(|w| w.end - w.start).sum();
        let mut processed = 0usize;
        let mut segments = Vec::new();
        let mut detected_language = None;
        for window in windows {
            let mut params = FullParams::new(whisper_rs::SamplingStrategy::BeamSearch {
                beam_size: conf
                    .beam_size
                    .unwrap_or(5)
                    .try_into()
                    .context("beam_size is too large")?,
                patience: conf.patience.unwrap_or(1.0),
            });
            // Built-in VAD concatenation is intentionally never enabled. It changes the
            // time base and the bundled C++ version also has the upstream #3711 bug.
            params.set_no_context(conf.vad.unwrap_or(true));
            params.set_print_special(false);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_timestamps(false);
            params.set_token_timestamps(false);
            params.set_temperature(conf.temperature.unwrap_or(0.0));
            let language = if self.lang == Language::Auto {
                detected_language.as_deref().unwrap_or("auto")
            } else {
                self.lang.as_str()
            };
            params.set_language(Some(language));
            if let Some(prompt) = &conf.initial_prompt {
                params.set_initial_prompt(prompt);
            }
            pb.set_message("Transcribing...");
            let progress = WindowProgress {
                progress: pb,
                completed: processed,
                window_samples: window.end - window.start,
                total_samples,
            };
            // whisper_full invokes callbacks synchronously and does not retain this pointer.
            unsafe {
                params.set_progress_callback(Some(report_progress));
                params.set_progress_callback_user_data(
                    &progress as *const WindowProgress<'_> as *mut c_void,
                );
            }
            let mut state = self.ctx.create_state()?;
            state.full(params, &audio.samples[window.start..window.end])?;
            if detected_language.is_none() {
                detected_language =
                    whisper_rs::get_lang_str(state.full_lang_id_from_state()).map(str::to_owned);
            }
            let begin = window.start_cs();
            let limit = window.end_cs();
            if limit <= begin {
                continue;
            }
            for segment in state.as_iter() {
                let text = segment.to_str_lossy()?.to_string();
                if text.trim().is_empty() {
                    continue;
                }
                let start = (begin + segment.start_timestamp()).clamp(begin, limit - 1);
                let end = (begin + segment.end_timestamp()).clamp(start + 1, limit);
                segments.push(TranscriptSegment { start, end, text });
            }
            processed += window.end - window.start;
            pb.set_position((100 * processed / total_samples) as u64);
        }
        if segments.is_empty() {
            bail!("No transcription segments found");
        }
        let language = if self.lang == Language::Auto {
            detected_language
                .as_deref()
                .context("Unable to determine alignment language")?
        } else {
            self.lang.as_str()
        };
        let stem = input
            .file_stem()
            .context("Input has no file name")?
            .to_string_lossy();
        crate::output::save_transcript_json(
            &input.with_file_name(format!("{stem}.raw.transcript.json")),
            &segments,
        )?;
        let result = alignment::refine(&audio, language, segments, &conf.alignment, pb)
            .context("Timing refinement failed; raw transcript has been saved")?;
        let report_path = input.with_file_name(format!("{stem}.timing.json"));
        std::fs::write(&report_path, serde_json::to_vec_pretty(&result.report)?)?;
        pb.finish_with_message("Transcribed and timing checked");
        Ok(result.segments)
    }
}

fn detect_windows(samples: &[f32]) -> Result<Vec<AudioWindow>> {
    // Silero VAD model (MIT License), Copyright (c) 2021 Silero Team.
    let mut model_file = NamedTempFile::new()?;
    model_file.write_all(include_bytes!("models/silero_vad.bin"))?;
    let mut vad = WhisperVadContext::new(
        model_file
            .path()
            .to_str()
            .context("Invalid VAD model path")?,
        WhisperVadContextParams::default(),
    )?;
    let mut params = WhisperVadParams::new();
    params.set_min_speech_duration(150);
    params.set_min_silence_duration(200);
    params.set_speech_pad(100);
    params.set_max_speech_duration(30.0);
    params.set_samples_overlap(0.0); // no concatenation and no overlapping sample copies
    let ranges = vad.segments_from_samples(params, samples)?;
    Ok(speech_windows(
        ranges.map(|r| (r.start, r.end)),
        samples.len(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffmpeg_decoder::SAMPLE_RATE;
    #[test]
    fn vad_on_silent_pcm_produces_no_transcription_windows() {
        assert!(detect_windows(&vec![0.0; SAMPLE_RATE * 2])
            .unwrap()
            .is_empty());
    }
}
