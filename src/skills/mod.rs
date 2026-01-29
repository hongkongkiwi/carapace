//! Skill management system.
//!
//! Supports importing and managing skills from:
//! - Carapace native skills (skill.yaml)
//! - Claude Code skills (CLAUDE.md)
//! - GitHub Actions (action.yml)

pub mod manifest;
pub mod registry;
pub mod import;
pub mod install;

pub use manifest::*;
pub use registry::*;
pub use import::*;
pub use install::*;

/// Skill installation result
#[derive(Debug, Clone)]
pub enum InstallResult {
    Installed {
        skill_id: String,
        path: std::path::PathBuf,
        components: Vec<String>,
    },
    Updated {
        skill_id: String,
        path: std::path::PathBuf,
        previous_version: String,
        new_version: String,
        changes: Vec<String>,
    },
    AlreadyInstalled {
        skill_id: String,
        path: std::path::PathBuf,
    },
}

/// Skill removal result
#[derive(Debug, Clone)]
pub enum RemoveResult {
    Removed {
        skill_id: String,
        backup_path: Option<std::path::PathBuf>,
    },
    NotFound {
        skill_id: String,
    },
}

/// Import source
#[derive(Debug, Clone, PartialEq)]
pub enum ImportSource {
    /// From GitHub (gh:owner/repo or https://github.com/owner/repo)
    GitHub { owner: String, repo: String },

    /// From a local directory
    Local { path: std::path::PathBuf },

    /// From a URL
    Url { url: String },

    /// From a marketplace index
    Marketplace { id: String },
}
