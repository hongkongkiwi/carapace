//! Tool Schema
//!
//! JSON Schema handling for tool parameters and validation.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Errors that can occur during schema validation
#[derive(Error, Debug)]
pub enum SchemaError {
    #[error("Invalid schema: {0}")]
    InvalidSchema(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Unknown type: {0}")]
    UnknownType(String),

    #[error("Missing required field: {0}")]
    MissingRequired(String),

    #[error("Type mismatch for field {field}: expected {expected}, got {actual}")]
    TypeMismatch {
        field: String,
        expected: String,
        actual: String,
    },
}

/// JSON Schema types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    Object,
    Array,
    String,
    Number,
    Integer,
    Boolean,
    Null,
}

/// Tool parameter schema
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParameterSchema {
    /// Schema type
    #[serde(rename = "type")]
    pub schema_type: Option<String>,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,

    /// Enum values (for string types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#enum: Option<Vec<Value>>,

    /// Object properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<std::collections::HashMap<String, ParameterSchema>>,

    /// Required fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,

    /// Array item schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<ParameterSchema>>,

    /// Minimum value (for numbers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,

    /// Maximum value (for numbers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,

    /// Minimum length (for strings/arrays)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,

    /// Maximum length (for strings/arrays)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,

    /// Pattern (for strings)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

impl ParameterSchema {
    /// Create a new string parameter
    pub fn string(description: impl Into<String>) -> Self {
        Self {
            schema_type: Some("string".to_string()),
            description: Some(description.into()),
            ..Default::default()
        }
    }

    /// Create a new integer parameter
    pub fn integer(description: impl Into<String>) -> Self {
        Self {
            schema_type: Some("integer".to_string()),
            description: Some(description.into()),
            ..Default::default()
        }
    }

    /// Create a new number parameter
    pub fn number(description: impl Into<String>) -> Self {
        Self {
            schema_type: Some("number".to_string()),
            description: Some(description.into()),
            ..Default::default()
        }
    }

    /// Create a new boolean parameter
    pub fn boolean(description: impl Into<String>) -> Self {
        Self {
            schema_type: Some("boolean".to_string()),
            description: Some(description.into()),
            ..Default::default()
        }
    }

    /// Create a new array parameter
    pub fn array(description: impl Into<String>, items: ParameterSchema) -> Self {
        Self {
            schema_type: Some("array".to_string()),
            description: Some(description.into()),
            items: Some(Box::new(items)),
            ..Default::default()
        }
    }

    /// Create a new object parameter
    pub fn object(description: impl Into<String>) -> Self {
        Self {
            schema_type: Some("object".to_string()),
            description: Some(description.into()),
            properties: Some(std::collections::HashMap::new()),
            ..Default::default()
        }
    }

    /// Set default value
    pub fn with_default(mut self, value: Value) -> Self {
        self.default = Some(value);
        self
    }

    /// Set enum values
    pub fn with_enum(mut self, values: Vec<Value>) -> Self {
        self.r#enum = Some(values);
        self
    }

    /// Add a property (for object types)
    pub fn with_property(
        mut self,
        name: impl Into<String>,
        schema: ParameterSchema,
    ) -> Self {
        if let Some(ref mut props) = self.properties {
            props.insert(name.into(), schema);
        }
        self
    }

    /// Set required fields
    pub fn with_required(mut self, fields: Vec<impl Into<String>>) -> Self {
        self.required = Some(fields.into_iter().map(|f| f.into()).collect());
        self
    }

    /// Set minimum value
    pub fn with_minimum(mut self, min: f64) -> Self {
        self.minimum = Some(min);
        self
    }

    /// Set maximum value
    pub fn with_maximum(mut self, max: f64) -> Self {
        self.maximum = Some(max);
        self
    }

    /// Convert to JSON Schema object
    pub fn to_json_schema(&self) -> Value {
        serde_json::to_value(self).unwrap_or(json!({}))
    }
}

/// Tool schema builder
pub struct ToolSchemaBuilder {
    schema: Value,
}

impl ToolSchemaBuilder {
    /// Create a new schema builder for an object type
    pub fn object() -> Self {
        Self {
            schema: json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    /// Add a property to the schema
    pub fn property(mut self, name: impl Into<String>, param: ParameterSchema) -> Self {
        let name = name.into();
        if let Some(props) = self.schema.get_mut("properties") {
            if let Some(obj) = props.as_object_mut() {
                obj.insert(name, param.to_json_schema());
            }
        }
        self
    }

    /// Set required fields
    pub fn required(mut self, fields: Vec<impl Into<String>>) -> Self {
        let fields: Vec<String> = fields.into_iter().map(|f| f.into()).collect();
        self.schema["required"] = json!(fields);
        self
    }

    /// Set a description for the parameters object
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.schema["description"] = json!(desc.into());
        self
    }

    /// Build the final schema
    pub fn build(self) -> Value {
        self.schema
    }
}

/// Validate parameters against a schema
pub fn validate_params(schema: &Value, params: &Value) -> Result<(), SchemaError> {
    // Check if params is an object
    let params_obj = params
        .as_object()
        .ok_or_else(|| SchemaError::ValidationFailed("Parameters must be an object".to_string()))?;

    // Check required fields
    if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
        for field in required {
            if let Some(field_name) = field.as_str() {
                if !params_obj.contains_key(field_name) {
                    return Err(SchemaError::MissingRequired(field_name.to_string()));
                }
            }
        }
    }

    // Validate properties
    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
        for (field_name, field_value) in params_obj {
            if let Some(field_schema) = properties.get(field_name) {
                validate_type(field_name, field_value, field_schema)?;
            }
        }
    }

    Ok(())
}

/// Validate a value against its schema type
fn validate_type(
    field_name: &str,
    value: &Value,
    schema: &Value,
) -> Result<(), SchemaError> {
    if let Some(expected_type) = schema.get("type").and_then(|t| t.as_str()) {
        let actual_type = get_json_type(value);

        if !type_matches(expected_type, &actual_type) {
            return Err(SchemaError::TypeMismatch {
                field: field_name.to_string(),
                expected: expected_type.to_string(),
                actual: actual_type,
            });
        }

        // Additional validation for strings
        if expected_type == "string" {
            if let Some(s) = value.as_str() {
                // Check pattern
                if let Some(pattern) = schema.get("pattern").and_then(|p| p.as_str()) {
                    // Simple pattern matching (could use regex crate)
                    if !s.contains(pattern.trim_start_matches('^').trim_end_matches('$')) {
                        return Err(SchemaError::ValidationFailed(format!(
                            "Field '{}' does not match pattern '{}'",
                            field_name, pattern
                        )));
                    }
                }

                // Check min/max length
                if let Some(min) = schema.get("minLength").and_then(|m| m.as_u64()) {
                    if s.len() < min as usize {
                        return Err(SchemaError::ValidationFailed(format!(
                            "Field '{}' is too short (min {})",
                            field_name, min
                        )));
                    }
                }

                if let Some(max) = schema.get("maxLength").and_then(|m| m.as_u64()) {
                    if s.len() > max as usize {
                        return Err(SchemaError::ValidationFailed(format!(
                            "Field '{}' is too long (max {})",
                            field_name, max
                        )));
                    }
                }

                // Check enum
                if let Some(enum_values) = schema.get("enum").and_then(|e| e.as_array()) {
                    let valid = enum_values.iter().any(|v| v.as_str() == Some(s));
                    if !valid {
                        return Err(SchemaError::ValidationFailed(format!(
                            "Field '{}' must be one of: {:?}",
                            field_name, enum_values
                        )));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Get the JSON type of a value
fn get_json_type(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(_) => "boolean".to_string(),
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "integer".to_string()
            } else {
                "number".to_string()
            }
        }
        Value::String(_) => "string".to_string(),
        Value::Array(_) => "array".to_string(),
        Value::Object(_) => "object".to_string(),
    }
}

/// Check if actual type matches expected type
fn type_matches(expected: &str, actual: &str) -> bool {
    match (expected, actual) {
        (e, a) if e == a => true,
        ("number", "integer") => true, // integer is a subtype of number
        _ => false,
    }
}

/// Convert a simple Rust type to JSON Schema
pub fn rust_type_to_schema<T: schemars::JsonSchema>() -> Value {
    let schema = schemars::schema_for!(T);
    serde_json::to_value(schema).unwrap_or(json!({}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_schema_builder() {
        let schema = ToolSchemaBuilder::object()
            .property("name", ParameterSchema::string("The name"))
            .property("age", ParameterSchema::integer("The age").with_minimum(0.0))
            .required(vec!["name"])
            .build();

        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
    }

    #[test]
    fn test_validate_params() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" }
            },
            "required": ["name"]
        });

        let valid = json!({"name": "John", "age": 30});
        assert!(validate_params(&schema, &valid).is_ok());

        let missing = json!({"age": 30});
        assert!(validate_params(&schema, &missing).is_err());

        let wrong_type = json!({"name": 123});
        assert!(validate_params(&schema, &wrong_type).is_err());
    }

    #[test]
    fn test_enum_validation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "color": {
                    "type": "string",
                    "enum": ["red", "green", "blue"]
                }
            }
        });

        let valid = json!({"color": "red"});
        assert!(validate_params(&schema, &valid).is_ok());

        let invalid = json!({"color": "yellow"});
        assert!(validate_params(&schema, &invalid).is_err());
    }
}

use serde_json::json;
