//! Skill manifest parsing and validation.
//!
//! Supports multiple skill formats:
//! - Carapace native skills (skill.yaml)
//! - Claude Code skills (CLAUDE.md + skill.json)
//! - GitHub Actions (action.yml)

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

pub mod carapace_skill;
pub mod claude_code;
pub mod github_action;

pub use carapace_skill::*;
pub use claude_code::*;
pub use github_action::*;

/// Errors that can occur during manifest parsing
#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid YAML: {0}")]
    InvalidYaml(#[from] serde_yaml::Error),

    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("Unknown skill format")]
    UnknownFormat,

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid version format: {0}")]
    InvalidVersion(String),

    #[error("Unsupported skill version: {0}")]
    UnsupportedVersion(String),
}

/// The type of skill source
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SkillSourceType {
    #[default]
    CarapaceSkill,
    ClaudeCodeSkill,
    GitHubAction,
}

/// Unified skill manifest that can represent any skill type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedSkillManifest {
    /// Unique identifier for the skill
    pub name: String,

    /// Human-readable description
    #[serde(default)]
    pub description: String,

    /// Semantic version
    #[serde(default)]
    pub version: String,

    /// Author information
    #[serde(default)]
    pub author: String,

    /// License identifier (SPDX format)
    #[serde(default)]
    pub license: String,

    /// Homepage URL
    #[serde(default)]
    pub homepage: String,

    /// Tags for discovery
    #[serde(default)]
    pub tags: Vec<String>,

    /// Source type detection
    #[serde(skip)]
    pub source_type: SkillSourceType,

    /// Carapace-specific extension
    #[serde(default)]
    pub carapace_ext: Option<CarapaceExtension>,

    /// Claude Code compatibility
    #[serde(default)]
    pub claude_code: Option<ClaudeCodeCompat>,

    /// GitHub Actions compatibility
    #[serde(default)]
    pub github_action: Option<GitHubActionsCompat>,

    /// Skill dependencies
    #[serde(default)]
    pub dependencies: Vec<SkillDependency>,
}

impl Default for UnifiedSkillManifest {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            version: String::new(),
            author: String::new(),
            license: String::new(),
            homepage: String::new(),
            tags: Vec::new(),
            source_type: SkillSourceType::CarapaceSkill,
            carapace_ext: None,
            claude_code: None,
            github_action: None,
            dependencies: Vec::new(),
        }
    }
}

/// Extension for Carapace-specific features
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CarapaceExtension {
    /// Path to agent configuration
    #[serde(default)]
    pub agent: String,

    /// Tool configuration globs
    #[serde(default)]
    pub tools: Vec<String>,

    /// Channel configuration globs
    #[serde(default)]
    pub channels: Vec<String>,

    /// Prompt file globs
    #[serde(default)]
    pub prompts: Vec<String>,
}

/// Dependency on another skill
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SkillDependency {
    /// Skill name or ID
    pub skill: String,

    /// Version constraint (e.g., ">=1.0.0", "~1.5.0")
    #[serde(default)]
    pub version: String,
}

/// Skill metadata (minimal info for listing)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillMetadata {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source_type: String,
    #[serde(default)]
    pub installed_at: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

/// Detect skill type from directory contents
pub async fn detect_skill_type(path: &Path) -> Result<SkillSourceType, ManifestError> {
    // Check for Carapace skill
    if path.join("skill.yaml").exists() {
        return Ok(SkillSourceType::CarapaceSkill);
    }

    // Check for GitHub Action
    if path.join("action.yml").exists() || path.join("action.yaml").exists() {
        return Ok(SkillSourceType::GitHubAction);
    }

    // Check for Claude Code skill
    if path.join("CLAUDE.md").exists() {
        return Ok(SkillSourceType::ClaudeCodeSkill);
    }

    // Check for skill.json (Claude Code alternative)
    if path.join("skill.json").exists() {
        return Ok(SkillSourceType::ClaudeCodeSkill);
    }

    Err(ManifestError::UnknownFormat)
}

/// Parse a skill manifest from a path
pub async fn parse_manifest(path: &Path) -> Result<UnifiedSkillManifest, ManifestError> {
    let source_type = detect_skill_type(path).await?;

    let manifest = match source_type {
        SkillSourceType::CarapaceSkill => {
            parse_carapace_skill(path).await?
        }
        SkillSourceType::ClaudeCodeSkill => {
            parse_claude_code_skill(path).await?
        }
        SkillSourceType::GitHubAction => {
            parse_github_action(path).await?
        }
    };

    Ok(manifest)
}

/// Validate a skill manifest
pub fn validate_manifest(manifest: &UnifiedSkillManifest) -> Result<(), ManifestError> {
    if manifest.name.is_empty() {
        return Err(ManifestError::MissingField("name".to_string()));
    }

    // Validate name format (lowercase, hyphens only)
    if !manifest.name.chars().all(|c| c.is_lowercase() || c == '-' || c == '_') {
        return Err(ManifestError::InvalidVersion(
            "name must contain only lowercase letters, hyphens, and underscores".to_string(),
        ));
    }

    // Validate version format if present
    if !manifest.version.is_empty() {
        validate_version(&manifest.version)?;
    }

    // At least one component must be specified
    let has_components = manifest.carapace_ext.is_some()
        || manifest.claude_code.is_some()
        || manifest.github_action.is_some();

    if !has_components {
        return Err(ManifestError::MissingField(
            "No skill components specified".to_string(),
        ));
    }

    Ok(())
}

/// Validate semantic version
fn validate_version(version: &str) -> Result<(), ManifestError> {
    // Simple semantic version check
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 2 {
        return Err(ManifestError::InvalidVersion(
            "version must be at least major.minor".to_string(),
        ));
    }

    // Check major version is numeric
    if parts[0].parse::<u64>().is_err() {
        return Err(ManifestError::InvalidVersion(
            "major version must be numeric".to_string(),
        ));
    }

    Ok(())
}

/// Get the manifest filename for a skill type
pub fn manifest_filename(source_type: &SkillSourceType) -> &'static str {
    match source_type {
        SkillSourceType::CarapaceSkill => "skill.yaml",
        SkillSourceType::ClaudeCodeSkill => "CLAUDE.md",
        SkillSourceType::GitHubAction => "action.yml",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_detect_carapace_skill() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();

        fs::write(path.join("skill.yaml"), "name: test-skill\ndescription: A test skill").unwrap();

        let result = detect_skill_type(path).await;
        assert_eq!(result.unwrap(), SkillSourceType::CarapaceSkill);
    }

    #[tokio::test]
    async fn test_detect_github_action() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();

        fs::write(path.join("action.yml"), "name: test-action").unwrap();

        let result = detect_skill_type(path).await;
        assert_eq!(result.unwrap(), SkillSourceType::GitHubAction);
    }

    #[tokio::test]
    async fn test_detect_claude_code_skill() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();

        fs::write(path.join("CLAUDE.md"), "# Test Skill").unwrap();

        let result = detect_skill_type(path).await;
        assert_eq!(result.unwrap(), SkillSourceType::ClaudeCodeSkill);
    }

    #[test]
    fn test_validate_version() {
        assert!(validate_version("1.0.0").is_ok());
        assert!(validate_version("0.1.0").is_ok());
        assert!(validate_version("2.0.0-beta.1").is_ok());

        assert!(validate_version("invalid").is_err());
        assert!(validate_version("v1.0.0").is_err());
    }

    #[test]
    fn test_validate_manifest_name() {
        let mut manifest = UnifiedSkillManifest::default();
        manifest.name = "valid-name".to_string();
        assert!(validate_manifest(&manifest).is_ok());

        manifest.name = "Invalid Name".to_string();
        assert!(validate_manifest(&manifest).is_err());
    }
}
