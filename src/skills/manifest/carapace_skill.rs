//! Carapace skill manifest parsing.

use super::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Carapace skill configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CarapaceSkillConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub compatibility: Option<Compatibility>,
    #[serde(default)]
    pub carapace: CarapaceExtension,
    #[serde(default)]
    pub dependencies: Vec<SkillDependency>,
}

/// Compatibility requirements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Compatibility {
    #[serde(default)]
    pub carapace: String,
}

/// Parse a Carapace skill manifest
pub async fn parse_carapace_skill(path: &Path) -> Result<UnifiedSkillManifest, ManifestError> {
    let manifest_path = path.join("skill.yaml");

    if !manifest_path.exists() {
        return Err(ManifestError::FileNotFound(manifest_path.to_string_lossy().to_string()));
    }

    let content = tokio::fs::read_to_string(&manifest_path)
        .await
        .map_err(|e| ManifestError::FileNotFound(e.to_string()))?;

    let config: CarapaceSkillConfig = serde_yaml::from_str(&content)?;

    Ok(UnifiedSkillManifest {
        name: config.name,
        description: config.description,
        version: config.version,
        author: config.author,
        license: config.license,
        homepage: config.homepage,
        tags: config.tags,
        source_type: SkillSourceType::CarapaceSkill,
        carapace_ext: Some(config.carapace),
        claude_code: None,
        github_action: None,
        dependencies: config.dependencies,
    })
}

/// Generate a skill.yaml from a manifest
pub fn generate_skill_yaml(manifest: &UnifiedSkillManifest) -> String {
    let config = CarapaceSkillConfig {
        name: manifest.name.clone(),
        description: manifest.description.clone(),
        version: manifest.version.clone(),
        author: manifest.author.clone(),
        license: manifest.license.clone(),
        homepage: manifest.homepage.clone(),
        tags: manifest.tags.clone(),
        compatibility: None,
        carapace: manifest.carapace_ext.clone().unwrap_or_default(),
        dependencies: manifest.dependencies.clone(),
    };

    serde_yaml::to_string(&config).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_parse_carapace_skill() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();

        let yaml = r#"
name: my-test-skill
description: A test skill for testing
version: 1.0.0
author: Test Author <test@example.com>
license: MIT

carapace:
  agent: agent.yaml
  tools:
    - tools/*.yaml
  prompts:
    - prompts/*.yaml

tags:
  - test
  - example
"#;

        tokio::fs::write(path.join("skill.yaml"), yaml).await.unwrap();

        let manifest = parse_carapace_skill(path).await.unwrap();

        assert_eq!(manifest.name, "my-test-skill");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.source_type, SkillSourceType::CarapaceSkill);
        assert!(manifest.carapace_ext.is_some());
    }
}
