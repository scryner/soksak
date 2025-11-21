mod config;
mod ffmpeg_decoder;
mod llm;
mod output;
mod transcribe;
mod translate;

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::{
    config::{Language, WhisperConfig},
    transcribe::Whisper,
};

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

        /// Configuration file for run options
        #[arg(short, long)]
        conf: Option<PathBuf>,

        /// Input language (default: auto)
        #[arg(short, long, default_value = "auto")]
        lang: Language,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { input, conf, lang } => {
            let app_config = config::load_app_config().context("Failed to load app config")?;

            let run_config = if let Some(conf_path) = conf {
                Some(config::load_run_config(&conf_path).context("Failed to load run config")?)
            } else {
                None
            };

            let input_path = input.canonicalize().context("Failed to find input file")?;
            let file_stem = input_path.file_stem().unwrap().to_string_lossy();
            let parent_dir = input_path.parent().unwrap();

            // 2. Transcribe
            println!("Transcribing...");
            let mut whisper = Whisper::new(&app_config.transcription, lang)
                .context("Failed to create Whisper instance")?;

            let whisper_conf = match &run_config {
                Some(config) => match &config.whisper {
                    Some(conf) => conf.clone(),
                    None => WhisperConfig::default(),
                },
                None => WhisperConfig::default(),
            };

            let mut pb = indicatif::ProgressBar::new(100);
            pb.set_style(
                indicatif::ProgressStyle::default_bar()
                    .template(
                        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}% ({eta})",
                    )
                    .unwrap()
                    .progress_chars("#>-"),
            );

            let segments = whisper
                .transcribe(&input_path, &whisper_conf, &mut pb)
                .context("Failed to transcribe")?;

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
                    pb_trans.set_style(indicatif::ProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                        .unwrap()
                        .progress_chars("#>-"));

                    let engine = if tc.llm.to_lowercase() == "apple" {
                        translate::TranslateEngine::Apple
                    } else {
                        translate::TranslateEngine::LLM
                    };

                    let translated_segments = translate::process_translation(
                        engine,
                        segments,
                        &app_config,
                        &tc.llm,
                        &tc.target_lang,
                        tc.window,
                        tc.system_prompt.as_deref().unwrap_or(""),
                        tc.filters.as_ref(),
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
    }

    Ok(())
}
