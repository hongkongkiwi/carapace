//! GitHub Action manifest parsing.

use super::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::collections::HashMap;

/// GitHub Action manifest (action.yml / action.yaml)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitHubActionManifest {
    /// Action name
    pub name: String,

    /// Action description
    #[serde(default)]
    pub description: String,

    /// Author
    #[serde(default)]
    pub author: String,

    /// Inputs
    #[serde(default)]
    pub inputs: HashMap<String, InputSpec>,

    /// Outputs
    #[serde(default)]
    pub outputs: HashMap<String, OutputSpec>,

    /// Runs configuration
    #[serde(default)]
    pub runs: Option<RunsSpec>,

    /// Branding (color, icon)
    #[serde(default)]
    pub branding: Option<BrandingSpec>,
}

/// Input specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InputSpec {
    pub description: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub deprecation_message: Option<String>,
}

/// Output specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OutputSpec {
    pub description: String,
    #[serde(default)]
    pub value: Option<String>,
}

/// Runs specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RunsSpec {
    /// Using: "node16", "node20", "docker", "composite"
    pub using: String,

    /// Main file
    #[serde(default)]
    pub main: String,

    /// Pre/brokencheck actions
    #[serde(default)]
    pub pre: Option<String>,

    /// Post actions
    #[serde(default)]
    pub post: Option<String>,

    /// Environment
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Branding specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BrandingSpec {
    pub color: String,
    pub icon: String,
}

/// GitHub Actions compatibility information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GitHubActionsCompat {
    /// Original manifest
    #[serde(skip)]
    pub manifest: GitHubActionManifest,

    /// Converted tool schema
    pub tool_schema: ToolSchema,
}

/// Tool schema converted from GitHub Action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolSchema {
    /// Tool name (derived from action name)
    pub name: String,

    /// Description
    pub description: String,

    /// Parameters (from inputs)
    pub parameters: Vec<ParameterSchema>,

    /// Return values (from outputs)
    pub returns: Vec<ReturnSchema>,
}

/// Parameter schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParameterSchema {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub r#type: String,
}

/// Return schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReturnSchema {
    pub name: String,
    pub description: String,
}

/// Parse a GitHub Action manifest
pub async fn parse_github_action(path: &Path) -> Result<UnifiedSkillManifest, ManifestError> {
    let manifest_path = if path.join("action.yml").exists() {
        path.join("action.yml")
    } else if path.join("action.yaml").exists() {
        path.join("action.yaml")
    } else {
        return Err(ManifestError::FileNotFound(
            "action.yml or action.yaml".to_string(),
        ));
    };

    let content = tokio::fs::read_to_string(&manifest_path)
        .await
        .map_err(|e| ManifestError::FileNotFound(e.to_string()))?;

    // Try YAML first, then JSON
    let manifest: GitHubActionManifest = match serde_yaml::from_str(&content) {
        Ok(m) => m,
        Err(_) => {
            // Try JSON
            serde_json::from_str(&content).map_err(|e| ManifestError::InvalidJson(e.into()))?
        }
    };

    let tool_schema = convert_to_tool_schema(&manifest);

    // Clone manifest first to avoid move issues
    let manifest_clone = manifest.clone();

    Ok(UnifiedSkillManifest {
        name: sanitize_action_name(&manifest.name),
        description: manifest.description,
        version: "1.0.0".to_string(), // GitHub Actions don't have version in manifest
        author: manifest.author,
        license: String::new(),
        homepage: String::new(),
        tags: vec!["github-action".to_string()],
        source_type: SkillSourceType::GitHubAction,
        carapace_ext: Some(CarapaceExtension {
            agent: String::new(),
            tools: vec!["converted/tool.yaml".to_string()],
            channels: vec![],
            prompts: vec![],
        }),
        claude_code: None,
        github_action: Some(GitHubActionsCompat {
            manifest: manifest_clone,
            tool_schema,
        }),
        dependencies: vec![],
    })
}

/// Sanitize action name for use as skill name
fn sanitize_action_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Convert GitHub Action manifest to tool schema
fn convert_to_tool_schema(manifest: &GitHubActionManifest) -> ToolSchema {
    let mut parameters = Vec::new();

    for (name, input) in &manifest.inputs {
        parameters.push(ParameterSchema {
            name: name.clone(),
            description: input.description.clone(),
            required: input.required,
            default: input.default.clone(),
            r#type: "string".to_string(), // Default to string
        });
    }

    let mut returns = Vec::new();

    for (name, output) in &manifest.outputs {
        returns.push(ReturnSchema {
            name: name.clone(),
            description: output.description.clone(),
        });
    }

    ToolSchema {
        name: sanitize_action_name(&manifest.name),
        description: manifest.description.clone(),
        parameters,
        returns,
    }
}

/// Generate a tool.yaml from GitHub Action
pub fn generate_tool_yaml(compat: &GitHubActionsCompat) -> String {
    let tool = &compat.tool_schema;

    let parameters: Vec<serde_json::Value> = tool
        .parameters
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "description": p.description,
                "type": p.r#type,
                "required": p.required,
            })
        })
        .collect();

    let returns: Vec<serde_json::Value> = tool
        .returns
        .iter()
        .map(|r| {
            serde_json::json!({
                "name": r.name,
                "description": r.description,
            })
        })
        .collect();

    // Create a default RunsSpec for when manifest.runs is None
    let default_runs = RunsSpec {
        using: "unknown".to_string(),
        main: String::new(),
        pre: None,
        post: None,
        env: HashMap::new(),
    };
    let runs = compat.manifest.runs.as_ref().unwrap_or(&default_runs);

    let json_value = serde_json::json!({
        "name": tool.name,
        "description": tool.description,
        "parameters": parameters,
        "returns": returns,
        "runner": {
            "type": runs.using,
            "main": runs.main,
        }
    });

    let yaml = serde_yaml::to_string(&json_value).unwrap_or_default();

    yaml
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_parse_github_action() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();

        let action_yaml = r#"
name: 'My GitHub Action'
description: A useful GitHub Action
author: 'Developer'

inputs:
  api_key:
    description: 'API key for authentication'
    required: true
  parameter:
    description: 'Some parameter'
    required: false
    default: 'default'

outputs:
  result:
    description: 'The result'

runs:
  using: 'node20'
  main: 'dist/index.js'
"#;

        tokio::fs::write(path.join("action.yml"), action_yaml).await.unwrap();

        let manifest = parse_github_action(path).await.unwrap();

        assert_eq!(manifest.source_type, SkillSourceType::GitHubAction);
        assert!(manifest.github_action.is_some());

        let tool = &manifest.github_action.as_ref().unwrap().tool_schema;
        assert_eq!(tool.parameters.len(), 2);
        assert_eq!(tool.returns.len(), 1);
    }

    #[test]
    fn test_sanitize_action_name() {
        assert_eq!(sanitize_action_name("My Action"), "my-action");
        assert_eq!(sanitize_action_name("Hello World!"), "hello-world");
        assert_eq!(sanitize_action_name("Test_123"), "test-123");
    }
}
