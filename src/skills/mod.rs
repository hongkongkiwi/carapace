//! Skills System
//!
//! Composable skill system for agent capabilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Skill definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Skill name
    pub name: String,
    /// Skill description
    pub description: String,
    /// Required tools
    #[serde(default)]
    pub required_tools: Vec<String>,
    /// Required permissions
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Skill configuration schema
    #[serde(default)]
    pub config_schema: Option<serde_json::Value>,
}

/// Skill registry
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Register a skill
    pub fn register(&mut self, skill: Skill) {
        self.skills.insert(skill.name.clone(), skill);
    }

    /// Get a skill by name
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// List all skills
    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_registry() {
        let mut registry = SkillRegistry::new();
        registry.register(Skill {
            name: "web_search".to_string(),
            description: "Search the web".to_string(),
            required_tools: vec!["web_search".to_string()],
            permissions: vec![],
            config_schema: None,
        });

        assert!(registry.get("web_search").is_some());
    }
}
