//! TTS Providers
//!
//! Text-to-speech provider implementations.

use super::{TtsConfig, TtsError, TtsProvider, TtsMode, Result};
use async_trait::async_trait;

/// TTS provider trait
#[async_trait]
pub trait TtsProviderTrait: Send + Sync {
    /// Synthesize speech from text
    async fn synthesize(&self, text: &str, voice: &str) -> Result<Vec<u8>>;

    /// Check if provider is available
    fn is_available(&self) -> bool;

    /// Get provider name
    fn name(&self) -> &str;

    /// List available voices
    async fn list_voices(&self) -> Result<Vec<Voice>>;
}

/// Voice information
#[derive(Debug, Clone)]
pub struct Voice {
    /// Voice ID
    pub id: String,
    /// Voice name
    pub name: String,
    /// Language code
    pub language: String,
    /// Gender
    pub gender: Option<String>,
}

/// Edge TTS provider (free, keyless)
pub struct EdgeTtsProvider;

impl EdgeTtsProvider {
    /// Create new Edge TTS provider
    pub fn new() -> Self {
        Self
    }
}

impl Default for EdgeTtsProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TtsProviderTrait for EdgeTtsProvider {
    async fn synthesize(&self, text: &str, voice: &str) -> Result<Vec<u8>> {
        // Edge TTS uses Microsoft Edge's online TTS service
        // This is a placeholder implementation
        // Full implementation would use the edge-tts protocol

        tracing::info!(
            text = %text,
            voice = %voice,
            "Edge TTS synthesis (placeholder)"
        );

        // Return empty bytes as placeholder
        // Real implementation would make HTTP request to Edge TTS service
        Ok(Vec::new())
    }

    fn is_available(&self) -> bool {
        true // Edge TTS is always available (no API key needed)
    }

    fn name(&self) -> &str {
        "edge"
    }

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        // Common Edge TTS voices
        Ok(vec![
            Voice {
                id: "en-US-AriaNeural".to_string(),
                name: "Aria".to_string(),
                language: "en-US".to_string(),
                gender: Some("Female".to_string()),
            },
            Voice {
                id: "en-US-GuyNeural".to_string(),
                name: "Guy".to_string(),
                language: "en-US".to_string(),
                gender: Some("Male".to_string()),
            },
            Voice {
                id: "en-GB-SoniaNeural".to_string(),
                name: "Sonia".to_string(),
                language: "en-GB".to_string(),
                gender: Some("Female".to_string()),
            },
        ])
    }
}

/// OpenAI TTS provider
pub struct OpenAiTtsProvider {
    api_key: String,
    client: reqwest::Client,
}

impl OpenAiTtsProvider {
    /// Create new OpenAI TTS provider
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl TtsProviderTrait for OpenAiTtsProvider {
    async fn synthesize(&self, text: &str, voice: &str) -> Result<Vec<u8>> {
        let response = self.client
            .post("https://api.openai.com/v1/audio/speech")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "model": "tts-1",
                "input": text,
                "voice": voice,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(TtsError::ProviderError(format!("OpenAI TTS error: {}", error)));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }

    fn name(&self) -> &str {
        "openai"
    }

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        Ok(vec![
            Voice {
                id: "alloy".to_string(),
                name: "Alloy".to_string(),
                language: "en".to_string(),
                gender: None,
            },
            Voice {
                id: "echo".to_string(),
                name: "Echo".to_string(),
                language: "en".to_string(),
                gender: None,
            },
            Voice {
                id: "fable".to_string(),
                name: "Fable".to_string(),
                language: "en".to_string(),
                gender: None,
            },
            Voice {
                id: "onyx".to_string(),
                name: "Onyx".to_string(),
                language: "en".to_string(),
                gender: None,
            },
            Voice {
                id: "nova".to_string(),
                name: "Nova".to_string(),
                language: "en".to_string(),
                gender: None,
            },
            Voice {
                id: "shimmer".to_string(),
                name: "Shimmer".to_string(),
                language: "en".to_string(),
                gender: None,
            },
        ])
    }
}

/// TTS manager
pub struct TtsManager {
    config: TtsConfig,
    provider: Box<dyn TtsProviderTrait>,
}

impl TtsManager {
    /// Create new TTS manager from config
    pub fn new(config: TtsConfig) -> Result<Self> {
        config.validate().map_err(|e| TtsError::ConfigError(e))?;

        let provider: Box<dyn TtsProviderTrait> = match config.provider {
            TtsProvider::Edge => Box::new(EdgeTtsProvider::new()),
            TtsProvider::OpenAi => {
                let api_key = config.openai_api_key.clone()
                    .ok_or_else(|| TtsError::ConfigError("OpenAI API key required".to_string()))?;
                Box::new(OpenAiTtsProvider::new(api_key))
            }
            _ => Box::new(EdgeTtsProvider::new()), // Fallback to Edge
        };

        Ok(Self { config, provider })
    }

    /// Check if TTS should be used for a message
    pub fn should_synthesize(&self, is_inbound: bool, is_tagged: bool) -> bool {
        if !self.config.enabled {
            return false;
        }

        match self.config.mode {
            TtsMode::Off => false,
            TtsMode::Always => true,
            TtsMode::Inbound => is_inbound,
            TtsMode::Tagged => is_tagged,
        }
    }

    /// Synthesize speech
    pub async fn synthesize(&self, text: &str) -> Result<Vec<u8>> {
        self.provider.synthesize(text, &self.config.voice).await
    }

    /// Get available voices
    pub async fn list_voices(&self) -> Result<Vec<Voice>> {
        self.provider.list_voices().await
    }

    /// Check if provider is available
    pub fn is_available(&self) -> bool {
        self.provider.is_available()
    }
}
