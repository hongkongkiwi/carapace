//! Security utilities module
//!
//! Provides functionality for:
//! - Redacting sensitive fields from configuration
//! - Write-only secret handling (GitHub-style)
//! - Security-related helpers
//! - Secure logging without secret leakage

use serde_json::{json, Value};

pub mod logging;

/// List of field names that should be redacted (case-insensitive)
const SENSITIVE_FIELD_NAMES: &[&str] = &[
    // API keys and tokens
    "apikey",
    "api_key",
    "token",
    "auth_token",
    "access_token",
    "refresh_token",
    "bearer_token",
    // Passwords and secrets
    "password",
    "secret",
    "client_secret",
    "app_secret",
    // Credentials
    "credential",
    "credentials",
    // Private keys
    "private_key",
    "privatekey",
    "secret_key",
    "secretkey",
    // Bot tokens (common in messaging platforms)
    "bot_token",
    "bottoken",
    "webhook_secret",
    "signing_secret",
    // Generic sensitive patterns
    "key", // Only in specific contexts (see is_likely_secret_context)
];

/// Context paths that indicate a field is sensitive
const SENSITIVE_CONTEXT_PATHS: &[&str] = &[
    "gateway.auth",
    "gateway.hooks",
    "hooks.token",
    "credentials",
    "secrets",
    "channels.telegram",
    "channels.discord",
    "channels.slack",
    "channels.whatsapp",
    "channels.line",
    "channels.matrix",
    "models.providers",
    "anthropic",
    "openai",
    "plugins",
];

/// Redaction placeholder
const REDACTED_PLACEHOLDER: &str = "***REDACTED***";

/// Check if a field name indicates sensitive content
fn is_sensitive_field_name(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    SENSITIVE_FIELD_NAMES.iter().any(|&s| name_lower == s.to_lowercase())
}

/// Check if we're in a context that likely contains secrets
fn is_likely_secret_context(path: &str) -> bool {
    let path_lower = path.to_lowercase();
    SENSITIVE_CONTEXT_PATHS.iter().any(|&ctx| path_lower.contains(&ctx.to_lowercase()))
}

/// Recursively redact sensitive fields from a JSON value
///
/// This function walks through the JSON structure and replaces any sensitive
/// field values with a placeholder. Sensitive fields are identified by their
/// names (e.g., "apiKey", "token", "password") and their context within the
/// configuration structure.
///
/// # Arguments
/// * `value` - The JSON value to redact
/// * `path` - The current path in the JSON structure (used for context)
///
/// # Returns
/// A new JSON value with sensitive fields redacted
///
/// # Example
/// ```
/// use serde_json::json;
/// use carapace::security::redact_secrets;
///
/// let config = json!({
///     "gateway": {
///         "port": 8080,
///         "auth": {
///             "token": "super-secret-token"
///         }
///     }
/// });
///
/// let redacted = redact_secrets(&config, "");
/// assert_eq!(redacted["gateway"]["auth"]["token"], "***REDACTED***");
/// assert_eq!(redacted["gateway"]["port"], 8080);
/// ```
pub fn redact_secrets(value: &Value, path: &str) -> Value {
    match value {
        Value::Object(map) => {
            let mut redacted = serde_json::Map::new();
            for (key, val) in map.iter() {
                let new_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };

                // Check if this field should be redacted
                if should_redact_field(key, &new_path, val) {
                    redacted.insert(key.clone(), Value::String(REDACTED_PLACEHOLDER.to_string()));
                } else {
                    // Recursively process nested values
                    redacted.insert(key.clone(), redact_secrets(val, &new_path));
                }
            }
            Value::Object(redacted)
        }
        Value::Array(arr) => {
            let redacted: Vec<Value> = arr
                .iter()
                .enumerate()
                .map(|(i, val)| redact_secrets(val, &format!("{}[{}]", path, i)))
                .collect();
            Value::Array(redacted)
        }
        // Primitive values are returned as-is
        _ => value.clone(),
    }
}

/// Determine if a field should be redacted based on its name, path, and value
fn should_redact_field(key: &str, path: &str, value: &Value) -> bool {
    // Only redact string values (secrets are typically strings)
    if !value.is_string() {
        return false;
    }

    // Check if the field name indicates sensitivity
    if is_sensitive_field_name(key) {
        // For generic "key" field, check context
        if key.eq_ignore_ascii_case("key") {
            return is_likely_secret_context(path);
        }
        return true;
    }

    // Check for URL patterns that might contain credentials
    if let Some(s) = value.as_str() {
        if looks_like_url_with_credentials(s) {
            return true;
        }
    }

    false
}

/// Check if a string looks like a URL containing credentials
fn looks_like_url_with_credentials(s: &str) -> bool {
    // Patterns like: https://user:pass@host, postgresql://user:pass@host, etc.
    let patterns = [
        "://",
        "@",
    ];

    if !patterns.iter().all(|&p| s.contains(p)) {
        return false;
    }

    // Check for credential-like patterns in URL
    // Look for :password@ or :token@ patterns
    if let Some(at_pos) = s.find('@') {
        let before_at = &s[..at_pos];
        if before_at.contains(':') {
            // Likely contains credentials
            return true;
        }
    }

    false
}

/// Redact secrets from configuration and return a safe version for API responses
///
/// This is the main entry point for redacting configuration before sending it
/// to clients. It ensures that sensitive fields like API keys and passwords
/// cannot be read back once set.
///
/// # Arguments
/// * `config` - The configuration value to redact
///
/// # Returns
/// A new configuration value with all sensitive fields replaced with "***REDACTED***"
pub fn redact_config_for_response(config: &Value) -> Value {
    redact_secrets(config, "")
}

/// Check if a configuration update would modify a sensitive field
///
/// This is useful for logging or audit purposes when sensitive fields are changed.
///
/// # Arguments
/// * `path` - The configuration path being updated (dot notation)
///
/// # Returns
/// true if the path points to a sensitive field
pub fn is_sensitive_path(path: &str) -> bool {
    let parts: Vec<&str> = path.split('.').collect();

    // Check if any component of the path is a sensitive field name
    for part in &parts {
        if is_sensitive_field_name(part) {
            return true;
        }
    }

    // Check if the path is within a sensitive context
    is_likely_secret_context(path)
}

/// Create a safe description of a configuration change for logging
///
/// This function creates a log-safe description of a config change that
/// doesn't leak secret values.
///
/// # Arguments
/// * `path` - The configuration path being updated
/// * `_value` - The new value (not used in output for security)
///
/// # Returns
/// A string describing the change safely
pub fn describe_config_change(path: &str, _value: &Value) -> String {
    if is_sensitive_path(path) {
        format!("Updated sensitive field at {}", path)
    } else {
        format!("Updated configuration at {}", path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_api_key() {
        let config = json!({
            "models": {
                "providers": {
                    "openai": {
                        "apiKey": "sk-secret123"
                    }
                }
            }
        });

        let redacted = redact_config_for_response(&config);
        assert_eq!(
            redacted["models"]["providers"]["openai"]["apiKey"],
            REDACTED_PLACEHOLDER
        );
    }

    #[test]
    fn test_redact_token() {
        let config = json!({
            "gateway": {
                "auth": {
                    "token": "super-secret"
                }
            }
        });

        let redacted = redact_config_for_response(&config);
        assert_eq!(
            redacted["gateway"]["auth"]["token"],
            REDACTED_PLACEHOLDER
        );
    }

    #[test]
    fn test_redact_password() {
        let config = json!({
            "gateway": {
                "auth": {
                    "password": "hunter2"
                }
            }
        });

        let redacted = redact_config_for_response(&config);
        assert_eq!(
            redacted["gateway"]["auth"]["password"],
            REDACTED_PLACEHOLDER
        );
    }

    #[test]
    fn test_redact_channel_tokens() {
        let config = json!({
            "channels": {
                "telegram": {
                    "token": "bot123:secret",
                    "enabled": true
                }
            }
        });

        let redacted = redact_config_for_response(&config);
        assert_eq!(
            redacted["channels"]["telegram"]["token"],
            REDACTED_PLACEHOLDER
        );
        assert_eq!(redacted["channels"]["telegram"]["enabled"], true);
    }

    #[test]
    fn test_preserve_non_sensitive_fields() {
        let config = json!({
            "gateway": {
                "port": 8080,
                "host": "localhost",
                "enabled": true
            }
        });

        let redacted = redact_config_for_response(&config);
        assert_eq!(redacted["gateway"]["port"], 8080);
        assert_eq!(redacted["gateway"]["host"], "localhost");
        assert_eq!(redacted["gateway"]["enabled"], true);
    }

    #[test]
    fn test_redact_url_with_credentials() {
        let config = json!({
            "database": {
                "url": "postgresql://user:password@localhost/db"
            }
        });

        let redacted = redact_config_for_response(&config);
        assert_eq!(
            redacted["database"]["url"],
            REDACTED_PLACEHOLDER
        );
    }

    #[test]
    fn test_redact_nested_secrets() {
        let config = json!({
            "plugins": {
                "my-plugin": {
                    "config": {
                        "apiKey": "secret-key",
                        "endpoint": "https://api.example.com"
                    }
                }
            }
        });

        let redacted = redact_config_for_response(&config);
        assert_eq!(
            redacted["plugins"]["my-plugin"]["config"]["apiKey"],
            REDACTED_PLACEHOLDER
        );
        assert_eq!(
            redacted["plugins"]["my-plugin"]["config"]["endpoint"],
            "https://api.example.com"
        );
    }

    #[test]
    fn test_redact_array_items() {
        let config = json!({
            "channels": {
                "list": [
                    { "name": "telegram", "token": "secret1" },
                    { "name": "discord", "token": "secret2" }
                ]
            }
        });

        let redacted = redact_config_for_response(&config);
        assert_eq!(
            redacted["channels"]["list"][0]["token"],
            REDACTED_PLACEHOLDER
        );
        assert_eq!(
            redacted["channels"]["list"][1]["token"],
            REDACTED_PLACEHOLDER
        );
        assert_eq!(
            redacted["channels"]["list"][0]["name"],
            "telegram"
        );
    }

    #[test]
    fn test_is_sensitive_path() {
        assert!(is_sensitive_path("gateway.auth.token"));
        assert!(is_sensitive_path("models.providers.openai.apiKey"));
        assert!(is_sensitive_path("channels.telegram.token"));
        assert!(is_sensitive_path("hooks.token"));
        assert!(!is_sensitive_path("gateway.port"));
        assert!(!is_sensitive_path("channels.telegram.enabled"));
    }

    #[test]
    fn test_describe_config_change() {
        let value = json!("secret");
        assert_eq!(
            describe_config_change("gateway.auth.token", &value),
            "Updated sensitive field at gateway.auth.token"
        );
        assert_eq!(
            describe_config_change("gateway.port", &value),
            "Updated configuration at gateway.port"
        );
    }
}
