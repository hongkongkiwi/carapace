//! Skill import from various sources.

use super::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Import result
#[derive(Debug, Clone)]
pub struct ImportResult {
    /// The imported manifest
    pub manifest: UnifiedSkillManifest,

    /// Where the skill was imported from
    pub source: ImportSource,

    /// Temporary path where skill was cloned
    pub temp_path: PathBuf,
}

/// Import a skill from a GitHub repository
pub async fn import_from_github(owner: &str, repo: &str) -> Result<ImportResult, ImportError> {
    let repo_url = format!("https://github.com/{}/{}", owner, repo);

    // Create a temporary directory
    let temp_dir = TempDir::new()
        .map_err(|e| ImportError::TempDir(e.to_string()))?;

    let temp_path = temp_dir.path().join(repo);

    // Clone the repository
    let output = Command::new("git")
        .args(&["clone", "--depth", "1", &repo_url, temp_path.to_str().unwrap()])
        .output()
        .map_err(|e| ImportError::GitClone(e.to_string()))?;

    if !output.status.success() {
        return Err(ImportError::GitClone(String::from_utf8_lossy(&output.stderr).to_string()));
    }

    // Parse the manifest
    let manifest = parse_manifest(&temp_path).await
        .map_err(|e| ImportError::ParseError(e.to_string()))?;

    Ok(ImportResult {
        manifest,
        source: ImportSource::GitHub {
            owner: owner.to_string(),
            repo: repo.to_string(),
        },
        temp_path: temp_path.to_path_buf(),
    })
}

/// Import a skill from a local directory
pub async fn import_from_local(path: &Path) -> Result<ImportResult, ImportError> {
    let manifest = parse_manifest(path).await
        .map_err(|e| ImportError::ParseError(e.to_string()))?;

    Ok(ImportResult {
        manifest,
        source: ImportSource::Local {
            path: path.to_path_buf(),
        },
        temp_path: path.to_path_buf(),
    })
}

/// Import from URL
pub async fn import_from_url(_url: &str) -> Result<ImportResult, ImportError> {
    // For now, just download and extract
    // This is a placeholder for more complex URL handling
    Err(ImportError::NotImplemented("URL import not yet implemented".to_string()))
}

/// Import from marketplace
pub async fn import_from_marketplace(_id: &str) -> Result<ImportResult, ImportError> {
    // This would fetch from a configured marketplace
    Err(ImportError::NotImplemented("Marketplace import not yet implemented".to_string()))
}

/// Import errors
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("Git clone failed: {0}")]
    GitClone(String),

    #[error("Failed to create temp directory: {0}")]
    TempDir(String),

    #[error("Failed to parse manifest: {0}")]
    ParseError(String),

    #[error("Unknown skill format")]
    UnknownFormat,

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

/// Parse an import source string
pub fn parse_import_source(source: &str) -> Result<ImportSource, String> {
    if source.starts_with("gh:") {
        let parts = source[3..].split('/').collect::<Vec<_>>();
        if parts.len() == 2 {
            return Ok(ImportSource::GitHub {
                owner: parts[0].to_string(),
                repo: parts[1].to_string(),
            });
        }
        return Err("Invalid GitHub format. Use gh:owner/repo".to_string());
    }

    if source.starts_with("http://") || source.starts_with("https://") {
        return Ok(ImportSource::Url {
            url: source.to_string(),
        });
    }

    // Assume it's a local path
    let path = Path::new(source);
    if path.exists() {
        return Ok(ImportSource::Local {
            path: path.to_path_buf(),
        });
    }

    Err(format!("Unknown import source: {}", source))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_parse_github_source() {
        assert!(parse_import_source("gh:owner/repo").is_ok());
        assert!(parse_import_source("https://github.com/owner/repo").is_ok());
    }

    #[tokio::test]
    async fn test_parse_local_source() {
        let dir = TempDir::new().unwrap();
        let result = parse_import_source(dir.path().to_str().unwrap());
        assert!(result.is_ok());
        if let ImportSource::Local { path } = result.unwrap() {
            assert!(path.exists());
        }
    }
}
