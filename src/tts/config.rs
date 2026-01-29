//! TTS Configuration
//!
//! Configuration for text-to-speech functionality.

use serde::{Deserialize, Serialize};

/// TTS mode for automatic speech generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtsMode {
    /// TTS is disabled
    #[default]
    Off,
    /// Always use TTS for all messages
    Always,
    /// Only use TTS for inbound messages
    Inbound,
    /// Only use TTS when explicitly tagged
    Tagged,
}

impl std::fmt::Display for TtsMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Off => write!(f, "off"),
            Self::Always => write!(f, "always"),
            Self::Inbound => write!(f, "inbound"),
            Self::Tagged => write!(f, "tagged"),
        }
    }
}

impl std::str::FromStr for TtsMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "always" => Ok(Self::Always),
            "inbound" => Ok(Self::Inbound),
            "tagged" => Ok(Self::Tagged),
            _ => Err(format!("Invalid TTS mode: {}", s)),
        }
    }
}

/// TTS provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtsProvider {
    /// Edge TTS (free, keyless)
    #[default]
    Edge,
    /// OpenAI TTS
    OpenAi,
    /// ElevenLabs TTS
    ElevenLabs,
}

/// TTS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    /// Enable TTS
    pub enabled: bool,
    /// Default TTS mode
    #[serde(default)]
    pub mode: TtsMode,
    /// Default TTS provider
    #[serde(default)]
    pub provider: TtsProvider,
    /// Default voice ID
    pub voice: String,
    /// Speech speed (0.5 to 2.0)
    #[serde(default = "default_speed")]
    pub speed: f32,
    /// Volume (0.0 to 1.0)
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// OpenAI API key (if using OpenAI provider)
    pub openai_api_key: Option<String>,
    /// ElevenLabs API key (if using ElevenLabs provider)
    pub elevenlabs_api_key: Option<String>,
    /// ElevenLabs voice ID
    pub elevenlabs_voice_id: Option<String>,
}

fn default_speed() -> f32 {
    1.0
}

fn default_volume() -> f32 {
    1.0
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: TtsMode::Off,
            provider: TtsProvider::Edge,
            voice: "en-US-AriaNeural".to_string(),
            speed: 1.0,
            volume: 1.0,
            openai_api_key: None,
            elevenlabs_api_key: None,
            elevenlabs_voice_id: None,
        }
    }
}

impl TtsConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        if self.speed < 0.5 || self.speed > 2.0 {
            return Err("Speed must be between 0.5 and 2.0".to_string());
        }

        if self.volume < 0.0 || self.volume > 1.0 {
            return Err("Volume must be between 0.0 and 1.0".to_string());
        }

        match self.provider {
            TtsProvider::OpenAi => {
                if self.openai_api_key.is_none() {
                    return Err("OpenAI API key required for OpenAI provider".to_string());
                }
            }
            TtsProvider::ElevenLabs => {
                if self.elevenlabs_api_key.is_none() {
                    return Err("ElevenLabs API key required for ElevenLabs provider".to_string());
                }
            }
            _ => {}
        }

        Ok(())
    }
}
