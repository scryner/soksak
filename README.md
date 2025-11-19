# soksak

## Overview
`soksak` is a command‑line tool for video transcription and optional translation.  
It uses Whisper for speech‑to‑text, supports configurable language detection, and can translate the resulting transcript using a large language model (LLM). The tool also provides progress bars for both transcription and translation steps.

## Features
- **Transcription** using Whisper with automatic or user‑specified language detection.  
- **Optional translation** of the transcript into a target language via an LLM.  
- Configurable via application‑wide (`app_config`) and run‑specific configuration files.  
- Generates output in JSON (transcript, translation) and SRT subtitle formats.  
- Progress indication with `indicatif` progress bars.

## Installation
```sh
# Clone the repository
git clone https://github.com/yourusername/soksak.git
cd soksak

# Build the project (requires Rust and Cargo)
cargo build --release
```

## Usage
```sh
# Basic transcription (auto language detection)
./target/release/soksak run -i <input_video_file>

# Transcription with explicit language
./target/release/soksak run -i <input_video_file> --lang en

# Transcription with a custom run configuration (including translation)
./target/release/soksak run -i <input_video_file> --conf <run_config.toml>
```

### Command‑line arguments
| Argument | Description |
|----------|-------------|
| `input`  | Path to the input video file (required). |
| `--conf, -c` | Optional path to a run‑specific configuration file (TOML). |
| `--lang, -l` | Input language (`auto` by default). Supported values are defined in `config::Language`. |

## Configuration

### Application configuration (`app_config`)
Located at the default config path (e.g., `~/.config/soksak/config.toml`). It contains global settings such as Whisper model parameters and default LLM credentials.

### Run configuration (`run_config.toml`)
A TOML file that can override Whisper settings and specify translation options:

```toml
[whisper]
# Whisper specific overrides (optional)

[translation]
llm = "openai"
target_lang = "en"
window = 10
filters = ["punctuation", "capitalization"]
```

## Output Files
After execution, the following files are generated in the same directory as the input video:

- `<filename>.transcript.json` – Raw transcription segments.  
- `<filename>.translation.json` – Translated segments (if translation is configured).  
- `<filename>.srt` – Subtitles in SRT format (if translation is configured).

## License
This project is licensed under the MIT License. See `LICENSE` for details.
