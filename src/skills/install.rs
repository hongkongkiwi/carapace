//! Install skill components to carapace.

use super::*;
use std::path::Path;
use std::fs;

/// Installation errors
#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("Skill not found: {0}")]
    NotFound(String),

    #[error("Failed to read skill file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to write file: {0}")]
    WriteError(String),

    #[error("Invalid skill component: {0}")]
    InvalidComponent(String),

    #[error("Conflict with existing skill: {0}")]
    Conflict(String),

    #[error("Failed to install agent: {0}")]
    AgentInstallError(String),

    #[error("Failed to install tool: {0}")]
    ToolInstallError(String),

    #[error("Failed to install channel: {0}")]
    ChannelInstallError(String),

    #[error("Failed to install prompt: {0}")]
    PromptInstallError(String),

    #[error("Registry error: {0}")]
    RegistryError(#[from] RegistryError),
}

/// Install a skill to carapace
pub async fn install_skill(
    registry: &SkillRegistry,
    manifest: &UnifiedSkillManifest,
    source_path: &Path,
    install_path: &Path,
) -> Result<InstallResult, InstallError> {
    let skill_id = SkillRegistry::generate_skill_id(
        &manifest.name,
        &manifest.version,
        match manifest.source_type {
            SkillSourceType::CarapaceSkill => "carapace",
            SkillSourceType::ClaudeCodeSkill => "claude-code",
            SkillSourceType::GitHubAction => "github-action",
        },
    );

    let install_path = install_path.join(&skill_id);

    // Create install directory
    tokio::fs::create_dir_all(&install_path).await?;

    // Install based on skill type
    let components = match manifest.source_type {
        SkillSourceType::CarapaceSkill => {
            install_carapace_skill(manifest, source_path, &install_path).await?
        }
        SkillSourceType::ClaudeCodeSkill => {
            install_claude_code_skill(manifest, source_path, &install_path).await?
        }
        SkillSourceType::GitHubAction => {
            install_github_action(manifest, source_path, &install_path).await?
        }
    };

    // Check if already installed
    if registry.get(&skill_id).await.is_some() {
        return Ok(InstallResult::AlreadyInstalled {
            skill_id,
            path: install_path,
        });
    }

    // Register the skill
    registry.register(manifest, install_path.clone(), components.clone()).await?;

    Ok(InstallResult::Installed {
        skill_id,
        path: install_path,
        components,
    })
}

/// Install a Carapace skill
async fn install_carapace_skill(
    manifest: &UnifiedSkillManifest,
    source_path: &Path,
    install_path: &Path,
) -> Result<Vec<String>, InstallError> {
    let mut components = Vec::new();

    // Install agent
    if let Some(ext) = &manifest.carapace_ext {
        if !ext.agent.is_empty() {
            let agent_source = source_path.join(&ext.agent);
            if agent_source.exists() {
                let agent_dest = install_path.join("agent.yaml");
                tokio::fs::copy(&agent_source, &agent_dest).await?;
                components.push("agent".to_string());
            }
        }

        // Install tools
        for tool_pattern in &ext.tools {
            let tools = glob::glob(tool_pattern)
                .map_err(|e| InstallError::InvalidComponent(e.to_string()))?;

            for tool_path in tools.flatten() {
                let dest = install_path.join("tools").join(tool_path.file_name().unwrap());
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                tokio::fs::copy(&tool_path, &dest).await?;
                components.push(format!("tool:{}", tool_path.file_name().unwrap().to_string_lossy()));
            }
        }

        // Install prompts
        for prompt_pattern in &ext.prompts {
            let prompts = glob::glob(prompt_pattern)
                .map_err(|e| InstallError::InvalidComponent(e.to_string()))?;

            for prompt_path in prompts.flatten() {
                let dest = install_path.join("prompts").join(prompt_path.file_name().unwrap());
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                tokio::fs::copy(&prompt_path, &dest).await?;
                components.push(format!("prompt:{}", prompt_path.file_name().unwrap().to_string_lossy()));
            }
        }
    }

    Ok(components)
}

/// Install a Claude Code skill
async fn install_claude_code_skill(
    manifest: &UnifiedSkillManifest,
    source_path: &Path,
    install_path: &Path,
) -> Result<Vec<String>, InstallError> {
    let mut components = Vec::new();

    // Convert CLAUDE.md to agent
    if let Some(cc) = &manifest.claude_code {
        let agent_dest = install_path.join("agent.yaml");
        let agent_content = format!(
            r#"name: {}
description: {}

---

{}

---
This agent was imported from a Claude Code skill.
"#,
            manifest.name,
            manifest.description,
            cc.system_prompt
        );
        tokio::fs::write(&agent_dest, agent_content).await?;
        components.push("agent".to_string());

        // Copy original CLAUDE.md
        let claude_md_dest = install_path.join("CLAUDE.md");
        tokio::fs::copy(source_path.join("CLAUDE.md"), &claude_md_dest).await?;
        components.push("claude-md".to_string());

        // Copy skill.json if exists
        if source_path.join("skill.json").exists() {
            tokio::fs::copy(source_path.join("skill.json"), install_path.join("skill.json")).await?;
            components.push("skill-json".to_string());
        }
    }

    Ok(components)
}

/// Install a GitHub Action
async fn install_github_action(
    manifest: &UnifiedSkillManifest,
    source_path: &Path,
    install_path: &Path,
) -> Result<Vec<String>, InstallError> {
    let mut components = Vec::new();

    // Generate tool.yaml from GitHub Action manifest
    if let Some(gh) = &manifest.github_action {
        let tool_yaml = generate_tool_yaml(gh);
        tokio::fs::write(install_path.join("tool.yaml"), tool_yaml).await?;
        components.push("tool".to_string());

        // Copy action.yml
        let action_src = source_path.join("action.yml");
        if action_src.exists() {
            tokio::fs::copy(&action_src, install_path.join("action.yml")).await?;
            components.push("action-yml".to_string());
        }
    }

    Ok(components)
}

/// Uninstall a skill
pub async fn uninstall_skill(
    registry: &SkillRegistry,
    skill_id: &str,
) -> Result<RemoveResult, InstallError> {
    let entry = registry.get(skill_id).await
        .ok_or_else(|| InstallError::NotFound(skill_id.to_string()))?;

    // Unregister from registry
    registry.unregister(skill_id).await?;

    // Remove files
    if entry.path.exists() {
        tokio::fs::remove_dir_all(&entry.path).await?;
    }

    Ok(RemoveResult::Removed {
        skill_id: skill_id.to_string(),
        backup_path: None,
    })
}

/// Update a skill
pub async fn update_skill(
    registry: &SkillRegistry,
    manifest: &UnifiedSkillManifest,
    source_path: &Path,
    skill_id: &str,
) -> Result<InstallResult, InstallError> {
    let entry = registry.get(skill_id).await
        .ok_or_else(|| InstallError::NotFound(skill_id.to_string()))?;

    let previous_version = entry.version.clone();
    let install_path = entry.path.clone();

    // Remove old files
    if install_path.exists() {
        tokio::fs::remove_dir_all(&install_path).await?;
    }

    // Reinstall
    let result = install_skill(registry, manifest, source_path, install_path.parent().unwrap()).await?;

    match result {
        InstallResult::Installed { skill_id, path, components } => {
            Ok(InstallResult::Updated {
                skill_id,
                path,
                previous_version,
                new_version: manifest.version.clone(),
                changes: components,
            })
        }
        _ => Err(InstallError::NotFound(skill_id.to_string())),
    }
}
