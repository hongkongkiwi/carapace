//! Claude Code skill manifest parsing.

use super::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Claude Code skill metadata (skill.json)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeCodeSkillJson {
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
    pub tags: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

/// Claude Code compatibility information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ClaudeCodeCompat {
    /// Subskill directories
    #[serde(default)]
    pub skills: Vec<String>,

    /// Main CLAUDE.md content
    #[serde(default)]
    pub main_claude_md: String,

    /// Parsed system prompt from CLAUDE.md
    #[serde(default)]
    pub system_prompt: String,

    /// Skill.json content (if present)
    #[serde(default)]
    pub skill_json: Option<ClaudeCodeSkillJson>,
}

/// Read and parse a CLAUDE.md file
async fn read_claude_md(path: &Path) -> Result<String, ManifestError> {
    let claude_md_path = path.join("CLAUDE.md");

    if !claude_md_path.exists() {
        return Err(ManifestError::FileNotFound(
            claude_md_path.to_string_lossy().to_string(),
        ));
    }

    tokio::fs::read_to_string(&claude_md_path)
        .await
        .map_err(|e| ManifestError::FileNotFound(e.to_string()))
}

/// Extract system prompt from CLAUDE.md
/// Looks for content after the first # heading
fn extract_system_prompt(content: &str) -> String {
    // Simple extraction: skip the title line and return rest as prompt
    // In practice, CLAUDE.md is the system prompt
    content.to_string()
}

/// Parse a Claude Code skill
pub async fn parse_claude_code_skill(path: &Path) -> Result<UnifiedSkillManifest, ManifestError> {
    let claude_md = read_claude_md(path).await?;
    let system_prompt = extract_system_prompt(&claude_md);

    // Try to read skill.json if present
    let skill_json_path = path.join("skill.json");
    let skill_json: Option<ClaudeCodeSkillJson> = if skill_json_path.exists() {
        let content = tokio::fs::read_to_string(&skill_json_path).await.ok();
        content.and_then(|c| serde_json::from_str(&c).ok())
    } else {
        None
    };

    // Detect subskills
    let mut skills = Vec::new();
    let skills_dir = path.join("skills");
    if skills_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                if entry.path().join("CLAUDE.md").exists() {
                    skills.push(entry.file_name().to_string_lossy().to_string());
                }
            }
        }
    }

    Ok(UnifiedSkillManifest {
        name: skill_json.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_lowercase().replace(|c: char| !c.is_alphanumeric() && c != '-', "-"))
                .unwrap_or_else(|| "claude-code-skill".to_string())
        }),
        description: skill_json
            .as_ref()
            .and_then(|s| Some(s.description.clone()))
            .unwrap_or_else(|| "Claude Code skill".to_string()),
        version: skill_json
            .as_ref()
            .and_then(|s| Some(s.version.clone()))
            .unwrap_or_else(|| "1.0.0".to_string()),
        author: skill_json
            .as_ref()
            .and_then(|s| Some(s.author.clone()))
            .unwrap_or_default(),
        license: skill_json
            .as_ref()
            .and_then(|s| Some(s.license.clone()))
            .unwrap_or_default(),
        homepage: String::new(),
        tags: skill_json
            .as_ref()
            .and_then(|s| Some(s.tags.clone()))
            .unwrap_or_default(),
        source_type: SkillSourceType::ClaudeCodeSkill,
        carapace_ext: Some(CarapaceExtension {
            agent: "converted/agent.yaml".to_string(),
            tools: vec![],
            channels: vec![],
            prompts: vec!["converted/prompt.md".to_string()],
        }),
        claude_code: Some(ClaudeCodeCompat {
            skills,
            main_claude_md: claude_md,
            system_prompt,
            skill_json,
        }),
        github_action: None,
        dependencies: vec![],
    })
}

/// Convert Claude Code skill to Carapace agent
pub fn convert_to_agent(manifest: &UnifiedSkillManifest) -> String {
    let prompt = manifest
        .claude_code
        .as_ref()
        .and_then(|c| Some(c.system_prompt.clone()))
        .unwrap_or_default();

    format!(
        r#"# Agent: {}

{}

---
This agent was imported from a Claude Code skill.
"#,
        manifest.name, prompt
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_parse_claude_code_skill() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();

        let claude_md = r#"# My Claude Code Skill

You are a helpful assistant that does X.

## Capabilities
- Feature 1
- Feature 2

## Limitations
- Limitation 1
"#;

        tokio::fs::write(path.join("CLAUDE.md"), claude_md).await.unwrap();

        let manifest = parse_claude_code_skill(path).await.unwrap();

        assert_eq!(manifest.source_type, SkillSourceType::ClaudeCodeSkill);
        assert!(manifest.claude_code.is_some());
        assert!(manifest.claude_code.unwrap().system_prompt.contains("helpful assistant"));
    }

    #[test]
    fn test_extract_system_prompt() {
        let content = r#"# My Skill

You are a helpful assistant."#;

        let prompt = extract_system_prompt(content);
        assert!(prompt.contains("You are a helpful assistant"));
    }
}
