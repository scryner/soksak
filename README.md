soksak
======

# Overview
`soksak` is a command-line tool for video/audio transcription and translation.  
It uses Whisper for speech-to-text and supports translation via LLM or Apple's Translation framework. The tool provides progress bars for both transcription and translation steps.

# Features
- **Transcription** using Whisper with automatic or user-specified language detection
- **Translation** using either LLM (OpenAI, Ollama, Claude, Gemini) or Apple's Translation framework
- **Post-processing** with customizable editing instructions and filtering
- **Two workflows**: Full pipeline (transcribe + translate) or translate-only from existing transcript
- Configurable via application-wide and run-specific YAML configuration files
- Generates output in JSON (transcript, translation) and SRT subtitle formats
- Progress indication with `indicatif` progress bars

# Installation
```sh
# Clone the repository
git clone https://github.com/scryner/soksak.git
cd soksak

# Build the project (requires Rust and Cargo)
cargo build --release
```

# Usage

## Run Command (Transcribe + Translate)
Transcribes a video/audio file and optionally translates the result.

```sh
# Basic transcription only (auto language detection)
soksak run <input_video_file>

# Transcription with explicit language
soksak run <input_video_file> --lang ja

# Transcription with translation (requires config file)
soksak run <input_video_file> --conf <config.yaml>

# Transcription with translation and specific language
soksak run <input_video_file> --conf <config.yaml> --lang ko
```

## Translate Command (Translation Only)
Translates an existing `.transcript.json` file without re-transcribing.

```sh
# Translate from existing transcript
soksak translate <input.transcript.json> --conf <config.yaml>

# Translate with specific source language
soksak translate <input.transcript.json> --conf <config.yaml> --lang ja
```

## Command-line Arguments

### `run` subcommand
| Argument | Description |
|----------|-------------|
| `input`  | Path to the input video/audio file (required) |
| `--conf, -c` | Optional path to a run-specific configuration file (YAML) |
| `--lang, -l` | Input language (default: `auto`). Use ISO 639-1 codes (e.g., `en`, `ja`, `ko`) |

### `translate` subcommand
| Argument | Description |
|----------|-------------|
| `input`  | Path to the `.transcript.json` file (required) |
| `--conf, -c` | Path to a run-specific configuration file (YAML, required) |
| `--lang, -l` | Source language (default: `auto`). Use ISO 639-1 codes |

# Configuration

## Application Configuration
Located at `~/.soksak/config.yaml`. Contains global settings for Whisper models and LLM providers.

**Transcription Engines:**
- `whisper_cpp`: Uses whisper.cpp for CPU/GPU-based transcription (available on all platforms)
- `whisperkit`: Uses Apple's WhisperKit for optimized transcription on macOS with Neural Engine support (requires `apple` feature flag)

**Example:**
```yaml
transcription:
  models:
    auto:
      engine: "whisper_cpp"
      model: "/path/to/whisper/model/ggml-large-v3.bin"
    en:
      engine: "whisper_cpp"
      model: "/path/to/whisper/model/ggml-base.en.bin"
    ja:
      engine: "whisperkit"  # macOS only, requires 'apple' feature
      model: "openai/whisper-large-v3"
    ko:
      engine: "whisper_cpp"
      model: "/path/to/whisper/model/ggml-large-v3.bin"

llm:
  providers:
    - id: "openai"
      api_type: "OpenAI"
      api_key: "sk-..."
      base_url: "https://api.openai.com/v1"
      json_mode_type: "JsonSchema"  # Options: JsonObject, JsonSchema, None
    
    - id: "ollama"
      api_type: "Ollama"
      base_url: "http://localhost:11434"
      json_mode_type: "JsonObject"
    
    - id: "claude"
      api_type: "Claude"
      api_key: "sk-ant-..."
      base_url: "https://api.anthropic.com"
      json_mode_type: "JsonSchema"
    
    - id: "gemini"
      api_type: "Gemini"
      api_key: "..."
      base_url: "https://generativelanguage.googleapis.com/v1beta"
      json_mode_type: "JsonSchema"
```

## Run Configuration
A YAML file that specifies Whisper overrides and translation settings.

### Configuration Fields

#### `whisper` (optional)
- `beam_size`: Beam search size (optional)
- `patience`: Patience parameter for beam search (optional)
- `initial_prompt`: Initial prompt to guide transcription (optional)
- `vad`: Enable Voice Activity Detection (optional, boolean)
- `temperature`: Temperature parameter for sampling (optional, float)

#### `translation.translate`
- `engine`: Translation engine configuration
  - **LLM engine:**
    - `type`: `"LLM"`
    - `model`: Provider and model (format: `"{provider_id}/{model}"`)
    - `system_prompt`: Custom system prompt for translation (optional)
    - `window`: Batch size for translation (default: 100)
  - **Apple engine:**
    - `type`: `"Apple"`
    - `window`: Batch size for translation (default: 100)
- `target_lang`: Target language code (ISO 639-1)

#### `translation.edit` (optional)
Post-processing configuration for translated text.

- `default_model`: Default LLM for editing (format: `"{provider_id}/{model}"`)
- `instructions`: List of editing instructions (optional)
- `filters`: List of filter configurations (optional)
  - `prompt`: Question to ask the LLM about each segment
  - `threshold`: Confidence threshold for filtering (optional)
  - `llm`: Specific LLM for this filter (optional, uses `default_model` if not specified)

**Example with LLM translation:**
```yaml
whisper:
  beam_size: 5
  patience: 1.0
  initial_prompt: "This is a technical presentation."
  vad: true
  temperature: 0.0

translation:
  translate:
    engine:
      type: "LLM"
      model: "openai/gpt-4"
      system_prompt: "Translate naturally and preserve technical terms."
      window: 100
    target_lang: "en"
  
  edit:
    default_model: "openai/gpt-4"
    instructions:
      - "Fix grammar and punctuation"
      - "Use formal tone"
    filters:
      - prompt: "Is this segment advertising or promotional content?"
        threshold: 0.7
        llm: "openai/gpt-4"
```

**Example with Apple Translation:**
```yaml
translation:
  translate:
    engine:
      type: "Apple"
      window: 100
    target_lang: "ko"
  
  edit:
    default_model: "openai/gpt-4"
    instructions:
      - "Improve readability"
```

# Output Files
After execution, the following files are generated in the same directory as the input:

- `<filename>.transcript.json` – Raw transcription segments with timestamps
- `<filename>.translation.json` – Translated segments (if translation is configured)
- `<filename>.srt` – Subtitles in SRT format (if translation is configured)

# License
This project is licensed under the MIT License. See `LICENSE` for details.
