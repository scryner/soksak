mod config;
mod ffmpeg_decoder;
mod llm;
mod output;
mod transcribe;
mod translate;

use anyhow::Context;
use clap::{Parser, Subcommand};
use config::Language;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::config::{TranscriptionEngine, WhisperConfig};
use crate::transcribe::whisper_cpp::Whisper;
#[cfg(feature = "apple")]
use crate::transcribe::whisperkit::WhisperKit;

#[derive(Parser)]
#[command(name = "soksak")]
#[command(about = "Video transcription and translation tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run transcription and optional translation
    Run {
        /// Input video file
        input: PathBuf,

        /// Configuration profile or file path
        #[arg(short, long)]
        profile: Option<String>,

        /// Input language (default: auto)
        #[arg(short, long, default_value = "auto")]
        lang: Language,
    },

    // Run translation
    Translate {
        /// Input video file: MUST BE json from 'Run' command
        input: PathBuf,

        /// Configuration profile or file path
        #[arg(short, long)]
        profile: String,

        /// Input language (default: auto)
        #[arg(short, long, default_value = "auto")]
        lang: Language,
    },
}

fn resolve_profile_path(profile: &str) -> anyhow::Result<PathBuf> {
    if profile.starts_with("~/") {
        let home = dirs::home_dir().context("Could not find home directory")?;
        return Ok(home.join(&profile[2..]));
    }

    let path = PathBuf::from(profile);
    if path.is_absolute() || profile.starts_with("./") || profile.starts_with("../") {
        return Ok(path);
    }

    let home = dirs::home_dir().context("Could not find home directory")?;
    Ok(home
        .join(".soksak/profiles")
        .join(format!("{}.yaml", profile)))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            input,
            profile,
            lang,
        } => {
            let app_config = config::load_app_config().context("Failed to load app config")?;

            let run_config = if let Some(p) = profile {
                let conf_path = resolve_profile_path(&p)?;
                Some(config::load_run_config(&conf_path).context("Failed to load run config")?)
            } else {
                None
            };

            let input_path = input.canonicalize().context("Failed to find input file")?;
            let file_stem = input_path.file_stem().unwrap().to_string_lossy();
            let parent_dir = input_path.parent().unwrap();

            // 2. Transcribe
            println!("Transcribing...");

            // Get model config for the specified language
            let model_config = app_config.transcription.models.get(&lang).ok_or_else(|| {
                anyhow::anyhow!("No transcription model configured for language: {:?}", lang)
            })?;

            let mut pb = indicatif::ProgressBar::new(100);
            pb.set_style(
                indicatif::ProgressStyle::default_bar()
                    .template(
                        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}% ({eta})",
                    )
                    .unwrap()
                    .progress_chars("#>-"),
            );

            let whisper_conf = match &run_config {
                Some(config) => match &config.whisper {
                    Some(conf) => conf.clone(),
                    None => WhisperConfig::default(),
                },
                None => WhisperConfig::default(),
            };

            let segments = match &model_config.engine {
                TranscriptionEngine::WhisperCpp => {
                    let mut whisper = Whisper::new(&app_config.transcription, lang.clone())
                        .await
                        .context("Failed to create Whisper instance")?;

                    whisper
                        .transcribe(&input_path, &whisper_conf, &mut pb)
                        .context("Failed to transcribe with WhisperCpp")?
                }
                #[cfg(feature = "apple")]
                TranscriptionEngine::Whisperkit => {
                    let lang_str = if lang == Language::Auto {
                        None
                    } else {
                        Some(lang.as_str())
                    };
                    let model_path = model_config.resolve_model_path().await?;
                    let whisperkit = WhisperKit::new(model_path.to_str().unwrap(), lang_str);
                    whisperkit
                        .transcribe(&input_path, &whisper_conf, &mut pb)
                        .context("Failed to transcribe with WhisperKit")?
                }
            };

            pb.finish_with_message("Transcription complete");

            // Save Transcript
            let transcript_path = parent_dir.join(format!("{}.transcript.json", file_stem));
            output::save_transcript_json(&transcript_path, &segments)?;
            println!("Saved transcript to {:?}", transcript_path);

            // 3. Translate (if config present)
            if let Some(rc) = run_config {
                if let Some(tc) = rc.translation {
                    println!("Translating...");
                    let pb_trans = indicatif::ProgressBar::new(segments.len() as u64);
                    pb_trans.set_style(
                        indicatif::ProgressStyle::default_bar()
                            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                            .unwrap()
                            .progress_chars("#>-"),
                    );
                    pb_trans.enable_steady_tick(Duration::from_millis(100));

                    let translated_segments = translate::process_translation(
                        &lang,
                        &tc.translate,
                        tc.edit.as_ref(),
                        segments,
                        &app_config,
                        &pb_trans,
                    )
                    .await?;
                    pb_trans.finish_with_message("Translation complete");

                    // Save Translation
                    let translation_path =
                        parent_dir.join(format!("{}.translation.json", file_stem));
                    output::save_translation_json(&translation_path, &translated_segments)?;
                    println!("Saved translation to {:?}", translation_path);

                    // Save SRT
                    let srt_path = parent_dir.join(format!("{}.srt", file_stem));
                    output::save_srt(&srt_path, &translated_segments)?;
                    println!("Saved SRT to {:?}", srt_path);
                }
            }
        }
        Commands::Translate {
            input,
            profile,
            lang,
        } => {
            println!("Translating from transcript: {:?}", input);

            // 1. Load App Config
            let app_config = config::load_app_config()?;

            // 2. Load Run Config (Required)
            let conf_path = resolve_profile_path(&profile)?;
            let run_config = config::load_run_config(&conf_path)?;
            let tc = run_config.translation.ok_or_else(|| {
                anyhow::anyhow!("Translation config is required for translate command")
            })?;

            // 3. Load Transcript
            let transcript_content = std::fs::read_to_string(&input)?;
            let segments: Vec<transcribe::TranscriptSegment> =
                serde_json::from_str(&transcript_content)?;

            if segments.is_empty() {
                anyhow::bail!("Transcript is empty");
            }

            // 4. Translate
            println!("Translating...");
            let pb_trans = ProgressBar::new(segments.len() as u64);
            pb_trans.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                    .unwrap()
                    .progress_chars("#>-"),
            );
            pb_trans.enable_steady_tick(Duration::from_millis(100));

            let translated_segments = translate::process_translation(
                &lang,
                &tc.translate,
                tc.edit.as_ref(),
                segments,
                &app_config,
                &pb_trans,
            )
            .await?;
            pb_trans.finish_with_message("Translation complete");

            // 5. Save Outputs
            let input_path = Path::new(&input);
            let raw_stem = input_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");

            let file_stem = if raw_stem.ends_with(".transcript") {
                raw_stem.trim_end_matches(".transcript")
            } else {
                raw_stem
            };
            let parent = input_path.parent().unwrap_or_else(|| Path::new("."));

            // Save JSON
            let output_json_path = parent.join(format!("{}.translation.json", file_stem));
            let json_file = std::fs::File::create(&output_json_path)?;
            serde_json::to_writer_pretty(json_file, &translated_segments)?;
            println!("Saved translation to {:?}", output_json_path);

            // Save SRT
            let output_srt_path = parent.join(format!("{}.srt", file_stem));
            let mut srt_file = std::fs::File::create(&output_srt_path)?;
            for (i, segment) in translated_segments.iter().enumerate() {
                writeln!(
                    srt_file,
                    "{}\n{} --> {}\n{}\n",
                    i + 1,
                    output::format_timestamp(segment.start),
                    output::format_timestamp(segment.end),
                    segment.translated
                )?;
            }
            println!("Saved SRT to {:?}", output_srt_path);
        }
    }

    Ok(())
}
