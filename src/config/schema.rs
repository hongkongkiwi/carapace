//! JSON Schema generation for configuration
//!
//! Provides JSON Schema generation for configuration validation and IDE support.

use schemars::schema::RootSchema;

/// Generate JSON Schema for the full configuration
pub fn generate_schema() -> RootSchema {
    schemars::schema_for!(super::types::Config)
}

/// Generate schema as a JSON string
pub fn generate_schema_json() -> String {
    let schema = generate_schema();
    serde_json::to_string_pretty(&schema).expect("Failed to serialize schema")
}

/// Generate schema with custom metadata
pub fn generate_schema_with_metadata() -> RootSchema {
    let mut schema = generate_schema();

    // Add schema metadata
    schema.schema.metadata().title = Some("Carapace Configuration".to_string());
    schema.schema.metadata().description =
        Some("Configuration schema for the carapace gateway".to_string());
    schema.schema.metadata().id =
        Some("https://carapace.dev/schemas/config/v1.json".to_string());

    schema
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_schema() {
        let schema = generate_schema();
        // Verify it's a valid schema
        assert!(
            schema.schema.metadata().title.is_some()
                || schema.schema.metadata().description.is_some()
        );
    }

    #[test]
    fn test_generate_schema_json() {
        let json = generate_schema_json();
        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("$schema").is_some() || parsed.get("title").is_some());
    }

    #[test]
    fn test_schema_with_metadata() {
        let schema = generate_schema_with_metadata();
        assert_eq!(
            schema.schema.metadata().title,
            Some("Carapace Configuration".to_string())
        );
    }
}
