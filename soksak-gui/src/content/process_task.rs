use anyhow::{Context, Result};
use config::{Language, TranscriptionEngine, WhisperConfig};
use rust_i18n::t;
use soksak_lib::progress::Progress;
use soksak_lib::{config, output, transcribe, translate};
use std::path::PathBuf;
use transcribe::whisper_cpp::Whisper;
#[cfg(target_os = "macos")]
use transcribe::whisperkit::WhisperKit;

pub async fn run_job(
    input_path: PathBuf,
    profile_name: String,
    progress: impl Progress + Clone + Send + Sync + 'static,
    cancel_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<()> {
    let (tx, rx) = async_channel::bounded(1);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();

        let result = match rt {
            Ok(rt) => rt.block_on(async {
                tokio::select! {
                    res = run_job_inner(input_path, profile_name, progress) => res,
                    _ = cancel_rx => Err(anyhow::anyhow!("Job canceled")),
                }
            }),
            Err(e) => Err(e.into()),
        };

        let _ = tx.send_blocking(result);
    });

    rx.recv().await?
}

async fn run_job_inner(
    input_path: PathBuf,
    profile_name: String,
    progress: impl Progress + Clone + Send + Sync + 'static,
) -> Result<()> {
    // Logic adapted from soksak-cli/src/main.rs
    let app_config = config::load_app_config().context("Failed to load app config")?;

    // specific profile loading logic...
    // In CLI: resolve_profile_path. Here we might assume profile_name is just the name or we need to find it.
    // The GUI passes 'profile name' (e.g. "Default").
    // We need to construct path ~/.soksak/profiles/<name>.yaml
    let home = dirs::home_dir().context("Could not find home directory")?;
    let conf_path = home
        .join(".soksak/profiles")
        .join(format!("{}.yaml", profile_name));

    let run_config = if conf_path.exists() {
        Some(config::load_run_config(&conf_path).context("Failed to load run config")?)
    } else {
        // Fallback or error? If "Default", maybe None?
        // CLI defaults to None if no profile arg.
        // But GUI always has a profile?
        // Let's assume None if file not found, but log it?
        None
    };

    let file_stem = input_path.file_stem().unwrap().to_string_lossy();
    let parent_dir = input_path.parent().unwrap();

    // 2. Transcribe
    progress.set_message(&t!("progress.transcribing"));

    let whisper_conf = match &run_config {
        Some(config) => match &config.whisper {
            Some(conf) => conf.clone(),
            None => WhisperConfig::default(),
        },
        None => WhisperConfig::default(),
    };

    // Default Language
    // GUI doesn't have an override arg for language yet, so use config or Auto
    let lang = whisper_conf.default_language.unwrap_or(Language::Auto);

    let model_config = app_config.transcription.models.get(&lang).ok_or_else(|| {
        anyhow::anyhow!("No transcription model configured for language: {:?}", lang)
    })?;

    // Need a separate progress for extraction?
    // CLI uses MultiProgress or nested.
    // GuiProgress sends events. We can just reuse it or create a sub-scope?
    // For now reuse.

    let segments = match &model_config.engine {
        TranscriptionEngine::WhisperCpp => {
            let mut whisper = Whisper::new(&app_config.transcription, lang.clone())
                .await
                .context("Failed to create Whisper instance")?;

            // CLI uses a separate bar for extraction. GuiProgress can handle it updates simply as "Extracting..." message updates.
            // But we pass &pb_extract and &pb (transcribe).
            // We can just pass same progress to both?

            whisper
                .transcribe(&input_path, &whisper_conf, &progress, &progress)
                .context("Failed to transcribe with WhisperCpp")?
        }
        #[cfg(target_os = "macos")]
        TranscriptionEngine::Whisperkit => {
            let lang_str = if lang == Language::Auto {
                None
            } else {
                Some(lang.as_str())
            };
            let model_path = model_config.resolve_model_path().await?;
            let whisperkit = WhisperKit::new(model_path.to_str().unwrap(), lang_str);

            whisperkit
                .transcribe(&input_path, &whisper_conf, &progress)
                .context("Failed to transcribe with WhisperKit")?
        }
        #[allow(unused)]
        _ => {
            anyhow::bail!("Unsupported transcription engine");
        }
    };

    // Save Transcript
    let transcript_path = parent_dir.join(format!("{}.transcript.json", file_stem));
    output::save_transcript_json(&transcript_path, &segments)?;

    // 3. Translate
    if let Some(rc) = run_config {
        if let Some(tc) = rc.translation {
            progress.set_position(0);
            progress.set_message(&t!("progress.translating"));
            // Translate progress needs valid length.
            // GuiProgress needs to handle `set_length`?
            // Progress trait has no set_length. It has `inc` and `set_position`.
            // CLI new ProgressBar creates with length.
            // But here we reuse. We probably should just `set_position(0)`?
            // `process_translation` takes `&impl Progress`.

            progress.set_length(segments.len() as u64);

            let translated_segments = translate::process_translation(
                &lang,
                &tc.translate,
                tc.edit.as_ref(),
                segments,
                &app_config,
                &progress,
            )
            .await?;

            // Save Translation
            let translation_path = parent_dir.join(format!("{}.translation.json", file_stem));
            output::save_translation_json(&translation_path, &translated_segments)?;

            let srt_path = parent_dir.join(format!("{}.srt", file_stem));
            output::save_srt(&srt_path, &translated_segments)?;
        }
    }

    // progress.finish_with_message("Done");
    Ok(())
}
