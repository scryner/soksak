use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Language {
    Auto,
    English,
    Chinese,
    German,
    Spanish,
    Russian,
    Korean,
    French,
    Japanese,
    Portuguese,
    Turkish,
    Polish,
    Catalan,
    Dutch,
    Arabic,
    Swedish,
    Italian,
    Indonesian,
    Hindi,
    Finnish,
    Vietnamese,
    Hebrew,
    Ukrainian,
    Greek,
    Malay,
    Czech,
    Romanian,
    Danish,
    Hungarian,
    Tamil,
    Norwegian,
    Thai,
    Urdu,
    Croatian,
    Bulgarian,
    Lithuanian,
    Latin,
    Maori,
    Malayalam,
    Welsh,
    Slovak,
    Telugu,
    Persian,
    Latvian,
    Bengali,
    Serbian,
    Azerbaijani,
    Slovenian,
    Kannada,
    Estonian,
    Macedonian,
    Breton,
    Basque,
    Icelandic,
    Armenian,
    Nepali,
    Mongolian,
    Bosnian,
    Kazakh,
    Albanian,
    Swahili,
    Galician,
    Marathi,
    Punjabi,
    Sinhala,
    Khmer,
    Shona,
    Yoruba,
    Somali,
    Afrikaans,
    Occitan,
    Georgian,
    Belarusian,
    Tajik,
    Sindhi,
    Gujarati,
    Amharic,
    Yiddish,
    Lao,
    Uzbek,
    Faroese,
    HaitianCreole,
    Pashto,
    Turkmen,
    Nynorsk,
    Maltese,
    Sanskrit,
    Luxembourgish,
    Myanmar,
    Tibetan,
    Tagalog,
    Malagasy,
    Assamese,
    Tatar,
    Hawaiian,
    Lingala,
    Hausa,
    Bashkir,
    Javanese,
    Sundanese,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Auto => "auto",
            Language::English => "en",
            Language::Chinese => "zh",
            Language::German => "de",
            Language::Spanish => "es",
            Language::Russian => "ru",
            Language::Korean => "ko",
            Language::French => "fr",
            Language::Japanese => "ja",
            Language::Portuguese => "pt",
            Language::Turkish => "tr",
            Language::Polish => "pl",
            Language::Catalan => "ca",
            Language::Dutch => "nl",
            Language::Arabic => "ar",
            Language::Swedish => "sv",
            Language::Italian => "it",
            Language::Indonesian => "id",
            Language::Hindi => "hi",
            Language::Finnish => "fi",
            Language::Vietnamese => "vi",
            Language::Hebrew => "he",
            Language::Ukrainian => "uk",
            Language::Greek => "el",
            Language::Malay => "ms",
            Language::Czech => "cs",
            Language::Romanian => "ro",
            Language::Danish => "da",
            Language::Hungarian => "hu",
            Language::Tamil => "ta",
            Language::Norwegian => "no",
            Language::Thai => "th",
            Language::Urdu => "ur",
            Language::Croatian => "hr",
            Language::Bulgarian => "bg",
            Language::Lithuanian => "lt",
            Language::Latin => "la",
            Language::Maori => "mi",
            Language::Malayalam => "ml",
            Language::Welsh => "cy",
            Language::Slovak => "sk",
            Language::Telugu => "te",
            Language::Persian => "fa",
            Language::Latvian => "lv",
            Language::Bengali => "bn",
            Language::Serbian => "sr",
            Language::Azerbaijani => "az",
            Language::Slovenian => "sl",
            Language::Kannada => "kn",
            Language::Estonian => "et",
            Language::Macedonian => "mk",
            Language::Breton => "br",
            Language::Basque => "eu",
            Language::Icelandic => "is",
            Language::Armenian => "hy",
            Language::Nepali => "ne",
            Language::Mongolian => "mn",
            Language::Bosnian => "bs",
            Language::Kazakh => "kk",
            Language::Albanian => "sq",
            Language::Swahili => "sw",
            Language::Galician => "gl",
            Language::Marathi => "mr",
            Language::Punjabi => "pa",
            Language::Sinhala => "si",
            Language::Khmer => "km",
            Language::Shona => "sn",
            Language::Yoruba => "yo",
            Language::Somali => "so",
            Language::Afrikaans => "af",
            Language::Occitan => "oc",
            Language::Georgian => "ka",
            Language::Belarusian => "be",
            Language::Tajik => "tg",
            Language::Sindhi => "sd",
            Language::Gujarati => "gu",
            Language::Amharic => "am",
            Language::Yiddish => "yi",
            Language::Lao => "lo",
            Language::Uzbek => "uz",
            Language::Faroese => "fo",
            Language::HaitianCreole => "ht",
            Language::Pashto => "ps",
            Language::Turkmen => "tk",
            Language::Nynorsk => "nn",
            Language::Maltese => "mt",
            Language::Sanskrit => "sa",
            Language::Luxembourgish => "lb",
            Language::Myanmar => "my",
            Language::Tibetan => "bo",
            Language::Tagalog => "tl",
            Language::Malagasy => "mg",
            Language::Assamese => "as",
            Language::Tatar => "tt",
            Language::Hawaiian => "haw",
            Language::Lingala => "ln",
            Language::Hausa => "ha",
            Language::Bashkir => "ba",
            Language::Javanese => "jw",
            Language::Sundanese => "su",
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Serialize for Language {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Language {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Language::from(s.as_str()))
    }
}

impl From<&str> for Language {
    fn from(s: &str) -> Self {
        match s {
            "auto" => Language::Auto,
            "en" => Language::English,
            "zh" => Language::Chinese,
            "de" => Language::German,
            "es" => Language::Spanish,
            "ru" => Language::Russian,
            "ko" => Language::Korean,
            "fr" => Language::French,
            "ja" => Language::Japanese,
            "pt" => Language::Portuguese,
            "tr" => Language::Turkish,
            "pl" => Language::Polish,
            "ca" => Language::Catalan,
            "nl" => Language::Dutch,
            "ar" => Language::Arabic,
            "sv" => Language::Swedish,
            "it" => Language::Italian,
            "id" => Language::Indonesian,
            "hi" => Language::Hindi,
            "fi" => Language::Finnish,
            "vi" => Language::Vietnamese,
            "he" => Language::Hebrew,
            "uk" => Language::Ukrainian,
            "el" => Language::Greek,
            "ms" => Language::Malay,
            "cs" => Language::Czech,
            "ro" => Language::Romanian,
            "da" => Language::Danish,
            "hu" => Language::Hungarian,
            "ta" => Language::Tamil,
            "no" => Language::Norwegian,
            "th" => Language::Thai,
            "ur" => Language::Urdu,
            "hr" => Language::Croatian,
            "bg" => Language::Bulgarian,
            "lt" => Language::Lithuanian,
            "la" => Language::Latin,
            "mi" => Language::Maori,
            "ml" => Language::Malayalam,
            "cy" => Language::Welsh,
            "sk" => Language::Slovak,
            "te" => Language::Telugu,
            "fa" => Language::Persian,
            "lv" => Language::Latvian,
            "bn" => Language::Bengali,
            "sr" => Language::Serbian,
            "az" => Language::Azerbaijani,
            "sl" => Language::Slovenian,
            "kn" => Language::Kannada,
            "et" => Language::Estonian,
            "mk" => Language::Macedonian,
            "br" => Language::Breton,
            "eu" => Language::Basque,
            "is" => Language::Icelandic,
            "hy" => Language::Armenian,
            "ne" => Language::Nepali,
            "mn" => Language::Mongolian,
            "bs" => Language::Bosnian,
            "kk" => Language::Kazakh,
            "sq" => Language::Albanian,
            "sw" => Language::Swahili,
            "gl" => Language::Galician,
            "mr" => Language::Marathi,
            "pa" => Language::Punjabi,
            "si" => Language::Sinhala,
            "km" => Language::Khmer,
            "sn" => Language::Shona,
            "yo" => Language::Yoruba,
            "so" => Language::Somali,
            "af" => Language::Afrikaans,
            "oc" => Language::Occitan,
            "ka" => Language::Georgian,
            "be" => Language::Belarusian,
            "tg" => Language::Tajik,
            "sd" => Language::Sindhi,
            "gu" => Language::Gujarati,
            "am" => Language::Amharic,
            "yi" => Language::Yiddish,
            "lo" => Language::Lao,
            "uz" => Language::Uzbek,
            "fo" => Language::Faroese,
            "ht" => Language::HaitianCreole,
            "ps" => Language::Pashto,
            "tk" => Language::Turkmen,
            "nn" => Language::Nynorsk,
            "mt" => Language::Maltese,
            "sa" => Language::Sanskrit,
            "lb" => Language::Luxembourgish,
            "my" => Language::Myanmar,
            "bo" => Language::Tibetan,
            "tl" => Language::Tagalog,
            "mg" => Language::Malagasy,
            "as" => Language::Assamese,
            "tt" => Language::Tatar,
            "haw" => Language::Hawaiian,
            "ln" => Language::Lingala,
            "ha" => Language::Hausa,
            "ba" => Language::Bashkir,
            "jw" => Language::Javanese,
            "su" => Language::Sundanese,
            _ => Language::Auto,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub transcription: TranscriptionConfig,
    pub llm: LlmConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TranscriptionConfig {
    pub models: HashMap<Language, String>,
}

#[derive(Debug, Deserialize)]
pub struct LlmConfig {
    pub providers: Vec<LlmProviderConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub enum ApiType {
    OpenAI,
    Ollama,
    Claude,
    Gemini,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LlmProviderConfig {
    pub id: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub api_type: ApiType,
}

#[derive(Debug, Deserialize)]
pub struct RunConfig {
    pub whisper: Option<WhisperConfig>,
    pub translation: Option<TranslationConfig>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct WhisperConfig {
    pub beam_size: Option<u32>,
    pub patience: Option<f32>,
    pub initial_prompt: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum TranslateEngine {
    LLM {
        model: String, // {provider_id}/{model}
        system_prompt: Option<String>,
        window: Option<usize>, // default size: 100
    },
    Apple {
        window: Option<usize>, // default size: 100
    },
}

#[derive(Debug, Deserialize, Clone)]
pub struct Translate {
    pub engine: TranslateEngine,
    pub target_lang: Language,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Edit {
    pub default_model: String, // {provider_id}/{model}
    pub instructions: Option<Vec<String>>,
    pub filters: Option<Vec<FilterConfig>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TranslationConfig {
    pub translate: Translate,
    pub edit: Option<Edit>, // Made optional as it might not always be present
}

#[derive(Debug, Deserialize, Clone)]
pub struct FilterConfig {
    pub prompt: String,
    pub threshold: Option<f32>,
    pub llm: Option<String>, // Optional specific LLM for this filter
}

pub fn load_app_config() -> anyhow::Result<AppConfig> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let config_path = home.join(".soksak/config.yaml");

    if !config_path.exists() {
        anyhow::bail!("Config file not found at {:?}", config_path);
    }

    let content = std::fs::read_to_string(config_path)?;
    let config: AppConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}

pub fn load_run_config(path: &PathBuf) -> anyhow::Result<RunConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: RunConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}
