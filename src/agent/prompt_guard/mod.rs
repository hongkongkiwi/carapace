//! Prompt Guard — defense-in-depth filtering for agent prompts and outputs.
//!
//! Four layers of protection:
//!
//! 1. **Pre-flight** — static analysis of system prompts for injection patterns
//! 2. **Tagging** — wraps untrusted content (tool results, fetched URLs) with delimiters
//! 3. **Post-flight** — filters PII, credentials, and harmful patterns from LLM output
//! 4. **Config lint** — detects risky agent configuration patterns

pub mod config_lint;
pub mod postflight;
pub mod preflight;
pub mod tagging;

use serde::{Deserialize, Serialize};

/// Top-level prompt guard configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptGuardConfig {
    /// Master switch — when `false`, all layers are skipped.
    #[serde(default)]
    pub enabled: bool,
    /// Pre-flight system prompt analysis.
    #[serde(default)]
    pub preflight: PreflightConfig,
    /// Untrusted content tagging.
    #[serde(default)]
    pub tagging: TaggingConfig,
    /// Post-flight output filtering.
    #[serde(default)]
    pub postflight: PostflightConfig,
    /// Agent configuration lint checks.
    #[serde(default)]
    pub config_lint: ConfigLintConfig,
}

/// Pre-flight layer configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Detect injection attempts ("ignore previous instructions", etc.)
    #[serde(default = "default_true")]
    pub detect_injection: bool,
    /// Detect privilege escalation ("bypass safety", "unrestricted mode")
    #[serde(default = "default_true")]
    pub detect_privilege_escalation: bool,
    /// Detect data exfiltration markers (markdown image injection, encoded URLs)
    #[serde(default = "default_true")]
    pub detect_exfiltration: bool,
}

impl Default for PreflightConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            detect_injection: true,
            detect_privilege_escalation: true,
            detect_exfiltration: true,
        }
    }
}

/// Tagging layer configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaggingConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for TaggingConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Post-flight layer configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostflightConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Block PII (email, phone, SSN, credit card) in output.
    #[serde(default = "default_true")]
    pub block_pii: bool,
    /// Block credential patterns (API keys, bearer tokens, password=) in output.
    #[serde(default = "default_true")]
    pub block_credentials: bool,
    /// Additional custom regex patterns to block.
    #[serde(default)]
    pub custom_patterns: Vec<String>,
}

impl Default for PostflightConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            block_pii: true,
            block_credentials: true,
            custom_patterns: Vec::new(),
        }
    }
}

/// Config lint layer configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigLintConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for ConfigLintConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

fn default_true() -> bool {
    true
}

/// Severity levels for prompt guard findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    /// Informational — logged but does not block.
    Info,
    /// Warning — logged, may block depending on configuration.
    Warning,
    /// Critical — blocks execution.
    Critical,
}

/// Category of a prompt guard finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingCategory {
    Injection,
    PrivilegeEscalation,
    Exfiltration,
    Pii,
    Credential,
    ConfigRisk,
}

impl PromptGuardConfig {
    /// Convenience accessor for custom postflight patterns.
    pub fn custom_patterns(&self) -> &[String] {
        &self.postflight.custom_patterns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_disabled() {
        let cfg = PromptGuardConfig::default();
        assert!(!cfg.enabled);
        assert!(cfg.preflight.enabled);
        assert!(cfg.tagging.enabled);
        assert!(cfg.postflight.enabled);
        assert!(cfg.config_lint.enabled);
    }

    #[test]
    fn test_config_serde_roundtrip() {
        let cfg = PromptGuardConfig {
            enabled: true,
            preflight: PreflightConfig {
                enabled: true,
                detect_injection: true,
                detect_privilege_escalation: false,
                detect_exfiltration: true,
            },
            tagging: TaggingConfig { enabled: false },
            postflight: PostflightConfig {
                enabled: true,
                block_pii: true,
                block_credentials: false,
                custom_patterns: vec!["secret_\\d+".to_string()],
            },
            config_lint: ConfigLintConfig { enabled: true },
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: PromptGuardConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.enabled, cfg.enabled);
        assert!(!parsed.postflight.block_credentials);
        assert_eq!(parsed.custom_patterns(), &["secret_\\d+"]);
    }

    #[test]
    fn test_default_preflight_all_enabled() {
        let cfg = PreflightConfig::default();
        assert!(cfg.enabled);
        assert!(cfg.detect_injection);
        assert!(cfg.detect_privilege_escalation);
        assert!(cfg.detect_exfiltration);
    }

    #[test]
    fn test_default_postflight_all_enabled() {
        let cfg = PostflightConfig::default();
        assert!(cfg.enabled);
        assert!(cfg.block_pii);
        assert!(cfg.block_credentials);
        assert!(cfg.custom_patterns.is_empty());
    }

    #[test]
    fn test_finding_severity_values() {
        assert_ne!(FindingSeverity::Info, FindingSeverity::Critical);
        assert_ne!(FindingSeverity::Warning, FindingSeverity::Critical);
    }

    #[test]
    fn test_finding_category_values() {
        assert_ne!(FindingCategory::Injection, FindingCategory::Pii);
        assert_ne!(FindingCategory::Credential, FindingCategory::Exfiltration);
    }

    #[test]
    fn test_config_deserialize_from_json5() {
        let json = r#"{"enabled": true, "preflight": {"enabled": true}}"#;
        let cfg: PromptGuardConfig = serde_json::from_str(json).unwrap();
        assert!(cfg.enabled);
        assert!(cfg.preflight.enabled);
    }

    #[test]
    fn test_config_deserialize_missing_fields_use_defaults() {
        let json = r#"{"enabled": true}"#;
        let cfg: PromptGuardConfig = serde_json::from_str(json).unwrap();
        assert!(cfg.enabled);
        assert!(cfg.preflight.enabled);
        assert!(cfg.tagging.enabled);
        assert!(cfg.postflight.enabled);
        assert!(cfg.config_lint.enabled);
    }
}
