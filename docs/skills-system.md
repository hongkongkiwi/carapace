# Skill System Design

## Overview

The carapace skill system enables importing skills from multiple sources with full compatibility with Claude Code skills and GitHub Actions.

## Skill Types

| Type | Manifest | Components | Import As |
|------|----------|------------|-----------|
| **Carapace Skill** | `skill.yaml` | agent, tools, channels, prompts | Direct install |
| **Claude Code Skill** | `CLAUDE.md` + `skills/` | Agent instructions | Agent + tools |
| **GitHub Action** | `action.yml` | Inputs, outputs, entrypoint | Tool plugin |

---

## Manifest Formats

### Carapace Skill Manifest (`skill.yaml`)

```yaml
# Required metadata
name: my-skill
description: A useful skill for X
version: 1.0.0
author: developer@example.com
license: MIT
homepage: https://github.com/developer/my-skill

# Compatibility (optional - for future use)
compatibility:
  carapace: ">=1.0.0"

# Carapace-specific components
carapace:
  agent: agent.yaml              # Agent configuration
  tools:                         # Tool definitions
    - tools/*.yaml
  channels:                      # Channel configurations
    - channels/*.yaml
  prompts:                       # System prompts
    - prompts/*.yaml

# Tags for discovery
tags:
  - productivity
  - automation
  - ai

# Dependencies on other skills
dependencies:
  - skill: other-skill
    version: ">=1.0.0"
```

### Claude Code Skill

```
claude-code-skill/
├── CLAUDE.md                    # Main instructions (REQUIRED)
├── skill.json                   # Metadata (optional)
└── skills/                      # Subskills (optional)
    ├── subskill1/
    │   └── CLAUDE.md
    └── subskill2/
        └── CLAUDE.md
```

**CLAUDE.md format**:
```markdown
# Skill Name

## Description
Brief description of what this skill does.

## Usage
How to use this skill.

## Capabilities
- Feature 1
- Feature 2

## Limitations
- Limitation 1
```

### GitHub Action (`action.yml`)

```yaml
name: 'My GitHub Action Tool'
description: 'A tool that does X'
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
    description: 'The result of the operation'

runs:
  using: 'node20'
  main: 'dist/index.js'
```

---

## Directory Structure

### Skill Registry (`~/.carapace/skills/`)

```
~/.carapace/skills/
├── registry.json                # Index of installed skills
└── installed/
    ├── carapace/
    │   └── my-carapace-skill-v1.0.0/
    │       ├── skill.yaml
    │       ├── agent.yaml
    │       ├── tools/
    │       ├── channels/
    │       └── prompts/
    ├── claude-code/
    │   └── my-claude-skill/
    │       ├── skill.json
    │       ├── CLAUDE.md
    │       └── converted/
    │           └── agent.yaml
    └── github-action/
        └── my-github-action/
            ├── action.yml
            └── converted/
                └── tool.yaml
```

### Installed Components (`~/.carapace/`)

After installation, skills install to their respective directories:

```
~/.carapace/
├── agents/
│   └── my-skill/
│       ├── config.yaml
│       └── system-prompt.md
├── tools/
│   └── my-tool/
│       ├── tool.yaml
│       └── impl.wasm
├── channels/
│   └── my-channel/
│       └── config.yaml
└── skills/
    └── my-skill.symlink -> ../skills/installed/carapace/my-skill/
```

---

## Architecture

### Module Structure

```
src/skills/
├── mod.rs                    # Public API
├── manifest.rs               # Manifest parsing (skill.yaml, action.yml, CLAUDE.md)
├── import.rs                 # Import from repo (git clone, GitHub API)
├── registry.rs               # Track installed skills
├── install.rs                # Install components to carapace
├── update.rs                 # Update installed skills
└── marketplace/
    ├── mod.rs
    └── fetch.rs              # Fetch skill list from marketplace
```

### Key Types

```rust
// src/skills/mod.rs
pub struct SkillManifest {
    pub name: String,
    pub description: String,
    pub version: String,
    pub source_type: SourceType,
    pub carapace_ext: Option<CarapaceExtension>,
    pub claude_code: Option<ClaudeCodeCompat>,
    pub github_action: Option<GitHubActionsCompat>,
}

pub enum SourceType {
    CarapaceSkill,
    ClaudeCodeSkill,
    GitHubAction,
}

pub enum InstallResult {
    Installed { skill_path: PathBuf },
    Updated { skill_path: PathBuf, previous_version: String },
    AlreadyInstalled,
}
```

---

## Import Process

### 1. Detect Source Type

```rust
async fn detect_source_type(path: &Path) -> Result<SourceType, Error> {
    if path.join("skill.yaml").exists() {
        return Ok(SourceType::CarapaceSkill);
    }
    if path.join("action.yml").exists() || path.join("action.yaml").exists() {
        return Ok(SourceType::GitHubAction);
    }
    if path.join("CLAUDE.md").exists() {
        return Ok(SourceType::ClaudeCodeSkill);
    }
    Err(Error::UnknownSkillFormat)
}
```

### 2. Parse Manifest

```rust
async fn parse_manifest(path: &Path) -> Result<SkillManifest, Error> {
    match detect_source_type(path).await? {
        SourceType::CarapaceSkill => {
            let content = tokio::fs::read_to_string(path.join("skill.yaml")).await?;
            Ok(serde_yaml::from_str(&content)?)
        }
        SourceType::GitHubAction => {
            // Convert GitHub Action to skill manifest
            convert_github_action(path).await
        }
        SourceType::ClaudeCodeSkill => {
            // Convert Claude Code skill to skill manifest
            convert_claude_code_skill(path).await
        }
    }
}
```

### 3. Install Components

```rust
async fn install_skill(manifest: &SkillManifest, path: &Path) -> Result<InstallResult> {
    match &manifest.source_type {
        SourceType::CarapaceSkill => {
            // Direct install from carapace skill structure
            install_carapace_skill(manifest, path).await
        }
        SourceType::ClaudeCodeSkill => {
            // Convert Claude Code → Carapace agent
            install_as_agent(manifest, path).await
        }
        SourceType::GitHubAction => {
            // Convert GitHub Action → Carapace tool
            install_as_tool(manifest, path).await
        }
    }
}
```

---

## CLI Commands

```bash
# Import from GitHub
carapace skills import gh:owner/repo

# Import from local directory
carapace skills import ./my-skill

# List installed skills
carapace skills list

# Update a skill
carapace skills update my-skill

# Remove a skill
carapace skills remove my-skill

# Search marketplace
carapace skills search "productivity"

# Browse marketplace
carapace skills browse
```

---

## API Endpoints

```
GET  /skills                    # List installed skills
POST /skills/import             # Import from URL
POST /skills/install            # Install from uploaded skill
GET  /skills/:id                # Get skill details
DELETE /skills/:id              # Remove skill
POST /skills/:id/update         # Update skill
GET  /skills/marketplace        # List marketplace skills
```

---

## Marketplace

The skill marketplace is a configurable GitHub repository containing:

```
marketplace-repo/
├── index.yaml                  # Skill index
└── skills/
    ├── skill-1/
    │   ├── skill.yaml
    │   └── README.md
    └── skill-2/
        ├── skill.yaml
        └── README.md
```

**index.yaml format**:
```yaml
skills:
  - id: skill-1
    name: Skill Name
    description: Short description
    version: 1.0.0
    tags: [productivity, automation]
    repo: gh:owner/skill-repo
    rating: 4.5
    downloads: 1000
```

---

## Security Considerations

1. **Code Review**: Imported skills should be reviewed before use
2. **Sandboxing**: WASM plugins for GitHub Actions
3. **Credential Handling**: Use credential store, never in skill config
4. **Network Access**: Skills can specify required capabilities
5. **Audit Logging**: Track skill installation and usage

---

## References

- [GitHub Actions metadata syntax](https://docs.github.com/en/actions/creating-actions/metadata-syntax-for-github-actions)
- [Claude Code skills documentation](https://docs.claude.com/)
- carapace agent configuration: `docs/agent-config.md`
- carapace plugin system: `src/plugins/bindings.rs`
