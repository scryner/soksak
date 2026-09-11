soksak
======

# Overview
`soksak` is a command-line tool for video/audio transcription and translation.  
It uses Whisper for speech-to-text and supports translation via LLM or Apple's Translation framework. The tool provides progress bars for both transcription and translation steps.

# Features
- **Transcription** using Whisper with automatic or user-specified language detection
- **Timing refinement** using local forced alignment while preserving the transcription text
- **Translation** using either LLM (OpenAI, Ollama, Claude, Gemini) or Apple's Translation framework
- **Post-processing** with customizable editing instructions and filtering
- **Two workflows**: Full pipeline (transcribe + translate) or translate-only from existing transcript
- Configurable via application-wide and run-specific YAML configuration files
- Generates output in JSON (transcript, translation) and SRT subtitle formats
- Progress indication with `indicatif` progress bars

# Build

## CLI
```sh
# Clone the repository
git clone https://github.com/scryner/soksak.git
cd soksak

# Build the project (requires Rust)
cargo build --release
```

## GUI
```sh
# Clone the repository
git clone https://github.com/scryner/soksak.git
cd soksak/soksak-gui

# Build the project (requires Rust and cargo-bundle)
cargo bundle --release
```

# Usage (CLI)

## Run Command (Transcribe + Translate)
Transcribes a video/audio file and optionally translates the result.

```sh
# Basic transcription only (auto language detection)
soksak run <input_video_file>

# Transcription with explicit language
soksak run <input_video_file> --lang ja

# Transcription with translation (requires config file)
soksak run <input_video_file> --profile ./config.yaml

# Transcription with translation and specific language
soksak run <input_video_file> --profile ./config.yaml --lang ko
```

## Translate Command (Translation Only)
Translates an existing `.transcript.json` file without re-transcribing.

```sh
# Translate from existing transcript
soksak translate <input.transcript.json> --profile ./config.yaml

# Translate with specific source language
soksak translate <input.transcript.json> --profile ./config.yaml --lang ja
```

## Command-line Arguments

### `run` subcommand
| Argument | Description |
|----------|-------------|
| `input`  | Path to the input video/audio file (required) |
| `--profile, -p` | Optional profile name in `~/.soksak/profiles/`, or a YAML path starting with `./`, `../`, `~/`, or `/` |
| `--lang, -l` | Input language (default: `auto`). Use ISO 639-1 codes (e.g., `en`, `ja`, `ko`) |

### `translate` subcommand
| Argument | Description |
|----------|-------------|
| `input`  | Path to the `.transcript.json` file (required) |
| `--profile, -p` | Profile name or YAML path (required) |
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
- `vad`: Detect and transcribe speech windows at their original audio positions (default: `true`)
- `temperature`: Temperature parameter for sampling (optional, float)
- `audio_stream`: Absolute audio stream index from `ffprobe` (optional; defaults to the marked default audio stream, then the first audio stream)
- `alignment`: Local timing refinement settings described below (WhisperCpp only)

### Improving subtitle timing (WhisperCpp)

The Whisper model remains responsible for transcription. FFmpeg decodes a 16 kHz mono
PCM timeline relative to the container's playback start, preserving delayed audio and
filling gaps in audio packet timestamps with silence. Both transcription and alignment
use these exact samples. FFmpeg **and ffprobe** must be installed (on macOS: `brew install ffmpeg`).

Silero VAD locates speech on that timeline. Each window is decoded from the original
PCM, and its absolute start is added to Whisper's timestamps. Short gaps within a window
remain intact; long gaps are skipped without compressing the timeline. This avoids the
built-in VAD timestamp remapping in the bundled whisper.cpp version, including
[the overlap mapping bug fixed upstream in #3711](https://github.com/ggml-org/whisper.cpp/pull/3711).
Changing VAD windows can affect Whisper's segmentation and recognition context; the
subsequent alignment step preserves text and segment count exactly.

Install the optional alignment runtime once:

```sh
python3 scripts/setup_alignment.py
```

With `uv` installed, the script creates a Python 3.12 environment. Otherwise, run it with
Python 3.10–3.13. The default location is `~/.soksak/alignment-venv`. This does not modify
system Python. WhisperX's language-specific CTC model aligns the existing text on the
CPU; its speech recognition and diarization pipelines are not used. Language models
download on first use, then use the local cache. Audio and transcripts stay local.

```yaml
whisper:
  vad: true
  # audio_stream: 1  # Absolute input stream index, NOT the audio-only ordinal
  alignment:
    mode: auto
    search_padding_seconds: 1.0
    max_shift_seconds: 2.0
    min_score: 0.3
    min_coverage: 0.8
    timeout_seconds: 1800
    # python: /absolute/path/to/venv/bin/python
    # model: organization/language-specific-ctc-model
```

These are the defaults, including when `alignment` is omitted from an existing profile.
`auto` attempts alignment and retains the original timing if the runtime/model is
unavailable. `required` stops the run on runtime/model/protocol failure, with the raw
transcript already saved. `off` skips alignment while keeping the extraction and VAD
timeline fixes. In both `auto` and `required`, individual uncertain segments retain
their original timing. Unsupported languages need a compatible CTC `model` override;
otherwise the selected failure policy applies. For multilingual audio, use an explicit
source language where possible; automatic detection selects the first window's language.

The Python executable is selected from `alignment.python`, then
`SOKSAK_ALIGNMENT_PYTHON`, then the default environment, then `python3` on PATH.
For a custom installation location, use `python3 scripts/setup_alignment.py --venv /path/to/venv`
and set `alignment.python` accordingly. Both CLI and GUI read these profile settings.

The aligner searches within a padded segment window bounded by neighboring transcript
midpoints. It requires scored first/last spoken characters, sufficient character coverage,
and a minimum mean acoustic score. This score is a heuristic, not a calibrated probability.
Invalid ranges, excessive shifts and new overlaps/order reversals are rejected. Search
windows over 90 seconds are left unchanged. The `.timing.json` report records each
segment's original and final times, score, coverage, and acceptance/fallback reason.

### Checking timing changes

Compare `.raw.transcript.json` against `.transcript.json` for the alignment-only change;
their text and segment count must match. For a VAD comparison, run separate profiles
with `vad: true` and `vad: false`, using `alignment.mode: off`, and preserve each run's
outputs before the next run. Output sidecar files are replaced when rerunning the same input.
Use representative clips with long pauses, quiet speech, music and overlapping voices.
Manually label speech starts/ends to measure median and 95th-percentile absolute boundary
error; a high alignment acceptance rate alone does not establish timing accuracy.

Regression checks (FFmpeg/ffprobe required):

```sh
cargo test -p soksak-lib -p soksak-cli
python3 scripts/test_alignment.py

# Optional real-model test; uses only the local model specified here.
SOKSAK_TEST_MODEL=/absolute/path/to/ggml-large-v3-turbo.bin \
  cargo test -p soksak-lib --test transcription_sync -- --ignored --nocapture --test-threads=1
```

The real-model tests insert a 30-second silence between repeated speech. With VAD enabled,
they check absolute offsets, text preservation and actual alignment acceptance. The VAD-off
control checks text preservation and the correction/fallback policy: without speech windows,
Whisper can anchor a caption at the beginning of a silent 30-second chunk. Corrections beyond
`max_shift_seconds` deliberately retain those raw times, so alignment alone cannot guarantee
good timing with VAD disabled. These are regression checks, not a benchmark against manually
annotated speech boundaries.

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

- `<filename>.raw.transcript.json` – WhisperCpp transcription before alignment
- `<filename>.timing.json` – WhisperCpp timing diagnostics and per-segment alignment decisions
- `<filename>.transcript.json` – Final transcription segments with refined timestamps (or original times where alignment was unavailable/rejected)
- `<filename>.translation.json` – Translated segments (if translation is configured)
- `<filename>.srt` – Subtitles in SRT format (if translation is configured)

JSON timestamps use centiseconds (1/100 second). SRT converts these to milliseconds once.
Translation retains the final transcription boundaries; SRT output rejects invalid or
reversed ranges before writing the file.

# License
This project is licensed under the MIT License. See `LICENSE` for details.
