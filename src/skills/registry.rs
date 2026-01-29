//! Skill registry - tracks installed skills.

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

/// Registry storage location
const REGISTRY_FILENAME: &str = "registry.json";

/// Installed skills storage directory
const INSTALLED_DIR: &str = "installed";

/// Registry errors
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Registry not initialized: {0}")]
    NotInitialized(PathBuf),

    #[error("Skill not found: {0}")]
    NotFound(String),

    #[error("Skill already installed: {0}")]
    AlreadyExists(String),

    #[error("Failed to read registry: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to write registry: {0}")]
    WriteError(#[from] serde_json::Error),

    #[error("Invalid registry format: {0}")]
    InvalidFormat(String),
}

/// Registry entry for an installed skill
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RegistryEntry {
    /// Unique skill ID (format: source-type/skill-name-version)
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Source type
    pub source_type: String,

    /// Version
    pub version: String,

    /// Source repository URL (if applicable)
    #[serde(default)]
    pub source_url: String,

    /// Installation path
    pub path: PathBuf,

    /// When the skill was installed
    pub installed_at: chrono::DateTime<chrono::Utc>,

    /// When the skill was last updated
    #[serde(default)]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Skill metadata from manifest
    #[serde(default)]
    pub metadata: SkillMetadata,

    /// Enabled/disabled state
    #[serde(default)]
    pub enabled: bool,

    /// Components installed
    #[serde(default)]
    pub components: Vec<String>,
}

/// The skill registry
#[derive(Debug)]
pub struct SkillRegistry {
    /// Registry data
    data: RwLock<RegistryData>,

    /// Base path for skill storage
    base_path: PathBuf,
}

/// Registry data stored on disk
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RegistryData {
    /// Version for future compatibility
    pub version: u32,

    /// All installed skills
    pub skills: HashMap<String, RegistryEntry>,
}

impl SkillRegistry {
    /// Create a new registry at the given path
    pub async fn new(base_path: PathBuf) -> Result<Self, RegistryError> {
        let registry_path = base_path.join(REGISTRY_FILENAME);
        let installed_path = base_path.join(INSTALLED_DIR);

        // Create directories if needed
        if !base_path.exists() {
            tokio::fs::create_dir_all(&base_path).await?;
        }
        if !installed_path.exists() {
            tokio::fs::create_dir_all(&installed_path).await?;
        }

        // Load existing registry or create new one
        let data = if registry_path.exists() {
            let content = tokio::fs::read_to_string(&registry_path).await?;
            serde_json::from_str(&content).map_err(|e| RegistryError::InvalidFormat(e.to_string()))?
        } else {
            RegistryData {
                version: 1,
                skills: HashMap::new(),
            }
        };

        Ok(Self {
            data: RwLock::new(data),
            base_path,
        })
    }

    /// Save the registry to disk
    pub async fn save(&self) -> Result<(), RegistryError> {
        let registry_path = self.base_path.join(REGISTRY_FILENAME);
        let content = serde_json::to_string_pretty(&*self.data.read().await)?;
        tokio::fs::write(&registry_path, content).await?;
        Ok(())
    }

    /// Generate a unique skill ID
    pub fn generate_skill_id(name: &str, version: &str, source_type: &str) -> String {
        format!(
            "{}/{}-{}",
            source_type,
            name.to_lowercase().replace(|c: char| !c.is_alphanumeric() && c != '-', "-"),
            version
        )
    }

    /// Register a skill
    pub async fn register(
        &self,
        manifest: &UnifiedSkillManifest,
        path: PathBuf,
        components: Vec<String>,
    ) -> Result<RegistryEntry, RegistryError> {
        let mut data = self.data.write().await;

        let skill_id = Self::generate_skill_id(
            &manifest.name,
            &manifest.version,
            match manifest.source_type {
                SkillSourceType::CarapaceSkill => "carapace",
                SkillSourceType::ClaudeCodeSkill => "claude-code",
                SkillSourceType::GitHubAction => "github-action",
            },
        );

        // Check for existing
        if data.skills.contains_key(&skill_id) {
            return Err(RegistryError::AlreadyExists(skill_id));
        }

        let entry = RegistryEntry {
            id: skill_id.clone(),
            name: manifest.name.clone(),
            source_type: match manifest.source_type {
                SkillSourceType::CarapaceSkill => "carapace".to_string(),
                SkillSourceType::ClaudeCodeSkill => "claude-code".to_string(),
                SkillSourceType::GitHubAction => "github-action".to_string(),
            },
            version: manifest.version.clone(),
            source_url: manifest.homepage.clone(),
            path: path.clone(),
            installed_at: chrono::Utc::now(),
            updated_at: None,
            metadata: SkillMetadata {
                name: manifest.name.clone(),
                description: manifest.description.clone(),
                version: manifest.version.clone(),
                author: manifest.author.clone(),
                tags: manifest.tags.clone(),
                source_type: match manifest.source_type {
                    SkillSourceType::CarapaceSkill => "carapace".to_string(),
                    SkillSourceType::ClaudeCodeSkill => "claude-code".to_string(),
                    SkillSourceType::GitHubAction => "github-action".to_string(),
                },
                installed_at: Some(chrono::Utc::now().to_rfc3339()),
                size: None,
            },
            enabled: true,
            components,
        };

        data.skills.insert(skill_id.clone(), entry.clone());

        // Save to disk
        drop(data);
        self.save().await?;

        Ok(entry)
    }

    /// Get a skill by ID
    pub async fn get(&self, id: &str) -> Option<RegistryEntry> {
        self.data.read().await.skills.get(id).cloned()
    }

    /// Get all skills
    pub async fn list(&self) -> Vec<RegistryEntry> {
        self.data.read().await.skills.values().cloned().collect()
    }

    /// Get enabled skills only
    pub async fn list_enabled(&self) -> Vec<RegistryEntry> {
        self.data
            .read()
            .await
            .skills
            .values()
            .filter(|e| e.enabled)
            .cloned()
            .collect()
    }

    /// Get skills by source type
    pub async fn list_by_source(&self, source_type: &str) -> Vec<RegistryEntry> {
        self.data
            .read()
            .await
            .skills
            .values()
            .filter(|e| e.source_type == source_type)
            .cloned()
            .collect()
    }

    /// Update a skill
    pub async fn update(
        &self,
        id: &str,
        manifest: &UnifiedSkillManifest,
        components: Vec<String>,
    ) -> Result<RegistryEntry, RegistryError> {
        let mut data = self.data.write().await;

        let entry = data
            .skills
            .get_mut(id)
            .ok_or_else(|| RegistryError::NotFound(id.to_string()))?;

        // Update the entry in place
        entry.version = manifest.version.clone();
        entry.metadata = SkillMetadata {
            name: manifest.name.clone(),
            description: manifest.description.clone(),
            version: manifest.version.clone(),
            author: manifest.author.clone(),
            tags: manifest.tags.clone(),
            source_type: entry.source_type.clone(),
            installed_at: entry.metadata.installed_at.clone(),
            size: None,
        };
        entry.updated_at = Some(chrono::Utc::now());
        entry.components = components;

        // Clone the entry after we're done modifying it
        let result = entry.clone();

        drop(data);
        self.save().await?;

        Ok(result)
    }

    /// Enable or disable a skill
    pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), RegistryError> {
        let mut data = self.data.write().await;

        let entry = data
            .skills
            .get_mut(id)
            .ok_or_else(|| RegistryError::NotFound(id.to_string()))?;

        entry.enabled = enabled;

        drop(data);
        self.save().await?;

        Ok(())
    }

    /// Unregister a skill
    pub async fn unregister(&self, id: &str) -> Result<RegistryEntry, RegistryError> {
        let mut data = self.data.write().await;

        let entry = data
            .skills
            .remove(id)
            .ok_or_else(|| RegistryError::NotFound(id.to_string()))?;

        drop(data);
        self.save().await?;

        Ok(entry)
    }

    /// Get the installation path for a skill
    pub fn get_install_path(&self, id: &str) -> PathBuf {
        self.base_path.join(INSTALLED_DIR).join(id)
    }

    /// Search skills by name or tag
    pub async fn search(&self, query: &str) -> Vec<RegistryEntry> {
        let query = query.to_lowercase();
        self.data
            .read()
            .await
            .skills
            .values()
            .filter(|e| {
                e.name.to_lowercase().contains(&query)
                    || e.metadata.description.to_lowercase().contains(&query)
                    || e.metadata.tags.iter().any(|t| t.to_lowercase().contains(&query))
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_registry_operations() {
        let dir = TempDir::new().unwrap();
        let registry = SkillRegistry::new(dir.path().to_path_buf()).await.unwrap();

        // Create a test manifest
        let manifest = UnifiedSkillManifest {
            name: "test-skill".to_string(),
            description: "A test skill".to_string(),
            version: "1.0.0".to_string(),
            source_type: SkillSourceType::CarapaceSkill,
            ..Default::default()
        };

        let path = dir.path().to_path_buf();
        let components = vec!["agent".to_string(), "tools".to_string()];

        // Register
        let entry = registry.register(&manifest, path.clone(), components.clone()).await.unwrap();
        assert_eq!(entry.name, "test-skill");
        assert!(entry.id.starts_with("carapace/"));

        // List
        let skills = registry.list().await;
        assert_eq!(skills.len(), 1);

        // Get
        let retrieved = registry.get(&entry.id).await.unwrap();
        assert_eq!(retrieved.name, "test-skill");

        // Search
        let results = registry.search("test").await;
        assert_eq!(results.len(), 1);

        // Disable
        registry.set_enabled(&entry.id, false).await.unwrap();
        let enabled = registry.list_enabled().await;
        assert_eq!(enabled.len(), 0);

        // Re-enable
        registry.set_enabled(&entry.id, true).await.unwrap();
        let enabled = registry.list_enabled().await;
        assert_eq!(enabled.len(), 1);

        // Unregister
        let removed = registry.unregister(&entry.id).await.unwrap();
        assert_eq!(removed.name, "test-skill");

        let skills = registry.list().await;
        assert!(skills.is_empty());
    }
}
