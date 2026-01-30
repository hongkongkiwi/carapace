//! Secure logging utilities
//!
//! Provides functions to sanitize log messages to prevent accidental
//! leakage of secrets and sensitive data.

use lazy_static::lazy_static;
use regex::Regex;

// Patterns that may indicate sensitive data in logs
lazy_static! {
    /// Pattern to match common secret formats in URLs and headers
    static ref SECRET_PATTERNS: Vec<(Regex, &'static str)> = vec![
        // Bearer tokens in Authorization headers
        (
            Regex::new(r"(?i)(authorization:\s*bearer\s+)[a-zA-Z0-9_\-.]+").unwrap(),
            "${1}***REDACTED***"
        ),
        // Basic auth in URLs: https://user:pass@host
        (
            Regex::new(r"(https?://[^:/@]+:)[^:/@]+(@)").unwrap(),
            "${1}***REDACTED***${2}"
        ),
        // API keys in query params
        (
            Regex::new(r"(?i)([?&](api_?key|token|secret|password)=)[^&]+").unwrap(),
            "${1}***REDACTED***"
        ),
        // JSON field patterns
        (
            Regex::new(r##"(?i)("(?:apiKey|api_key|token|secret|password|privateKey|private_key|bearer|authorization)":\s*")[^"]+"##).unwrap(),
            "${1}***REDACTED***"
        ),
        // Bot tokens (common formats)
        (
            Regex::new(r"(\d{9,10}:[a-zA-Z0-9_-]{35,})").unwrap(),
            "***BOT_TOKEN_REDACTED***"
        ),
        // OpenAI API keys
        (
            Regex::new(r"(sk-[a-zA-Z0-9]{48})").unwrap(),
            "***OPENAI_KEY_REDACTED***"
        ),
        // Anthropic API keys
        (
            Regex::new(r"(sk-ant-[a-zA-Z0-9_-]{32,})").unwrap(),
            "***ANTHROPIC_KEY_REDACTED***"
        ),
        // Generic private keys
        (
            Regex::new(r"(-----BEGIN (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----)[\s\S]+?(-----END (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----)").unwrap(),
            "${1}\n***PRIVATE_KEY_REDACTED***\n${2}"
        ),
    ];
}

/// Sanitize a log message to remove sensitive data
///
/// This function applies multiple regex patterns to detect and redact
/// common secret formats including:
/// - API keys and tokens
/// - Passwords in URLs
/// - Authorization headers
/// - Private keys
/// - Bot tokens
///
/// # Examples
///
/// ```
/// use carapace::security::logging::sanitize_log_message;
///
/// let message = "Authorization: Bearer sk-abc123xyz789";
/// let sanitized = sanitize_log_message(message);
/// assert!(sanitized.contains("***REDACTED***"));
/// ```
pub fn sanitize_log_message(message: &str) -> String {
    let mut result = message.to_string();

    for (pattern, replacement) in SECRET_PATTERNS.iter() {
        result = pattern.replace_all(&result, *replacement).to_string();
    }

    result
}

/// Sanitize an error message to remove sensitive data
///
/// Similar to `sanitize_log_message` but also handles error-specific
/// patterns like connection strings and stack traces that might
/// contain sensitive information.
pub fn sanitize_error_message(error: &str) -> String {
    let mut result = sanitize_log_message(error);

    // Additional patterns specific to error messages
    lazy_static! {
        static ref ERROR_PATTERNS: Vec<(Regex, &'static str)> = vec![
            // Connection strings with passwords
            (
                Regex::new(r"(?i)(postgres(?:ql)?://[^:/@]+:)[^:/@]+(@[^/]+)").unwrap(),
                "${1}***REDACTED***${2}"
            ),
            (
                Regex::new(r"(?i)(mysql://[^:/@]+:)[^:/@]+(@[^/]+)").unwrap(),
                "${1}***REDACTED***${2}"
            ),
            (
                Regex::new(r"(?i)(mongodb(?:\+srv)?://[^:/@]+:)[^:/@]+(@[^/]+)").unwrap(),
                "${1}***REDACTED***${2}"
            ),
            // Redis URLs with passwords
            (
                Regex::new(r"(?i)(redis://:[^@]+@)").unwrap(),
                "redis://:***REDACTED***@"
            ),
        ];
    }

    for (pattern, replacement) in ERROR_PATTERNS.iter() {
        result = pattern.replace_all(&result, *replacement).to_string();
    }

    result
}

/// A wrapper type that automatically sanitizes its content when displayed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedString {
    original: String,
}

impl SanitizedString {
    /// Create a new sanitized string from a potentially sensitive value
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            original: value.into(),
        }
    }

    /// Get the sanitized version of the string
    pub fn sanitized(&self) -> String {
        sanitize_log_message(&self.original)
    }

    /// Get the original value (be careful with this!)
    pub fn original(&self) -> &str {
        &self.original
    }
}

impl std::fmt::Display for SanitizedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.sanitized())
    }
}

impl From<String> for SanitizedString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for SanitizedString {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Extension trait for sanitizing strings
pub trait SanitizeExt {
    /// Sanitize this string for logging
    fn sanitize_for_logs(&self) -> String;

    /// Sanitize this string for error messages
    fn sanitize_for_errors(&self) -> String;
}

impl SanitizeExt for str {
    fn sanitize_for_logs(&self) -> String {
        sanitize_log_message(self)
    }

    fn sanitize_for_errors(&self) -> String {
        sanitize_error_message(self)
    }
}

impl SanitizeExt for String {
    fn sanitize_for_logs(&self) -> String {
        sanitize_log_message(self)
    }

    fn sanitize_for_errors(&self) -> String {
        sanitize_error_message(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_bearer_token() {
        let message = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let sanitized = sanitize_log_message(message);
        assert!(sanitized.contains("***REDACTED***"));
        assert!(!sanitized.contains("eyJhbGci"));
    }

    #[test]
    fn test_sanitize_url_password() {
        let message = "Connecting to https://user:secretpassword@example.com/db";
        let sanitized = sanitize_log_message(message);
        assert!(sanitized.contains("***REDACTED***"));
        assert!(!sanitized.contains("secretpassword"));
    }

    #[test]
    fn test_sanitize_api_key_query() {
        let message = "GET /api/data?api_key=abc123xyz&user=john";
        let sanitized = sanitize_log_message(message);
        assert!(sanitized.contains("***REDACTED***"));
        assert!(!sanitized.contains("abc123xyz"));
    }

    #[test]
    fn test_sanitize_json_field() {
        let message = r#"{"apiKey": "sk-live-1234567890abcdef", "name": "test"}"#;
        let sanitized = sanitize_log_message(message);
        assert!(sanitized.contains("***REDACTED***"));
        assert!(!sanitized.contains("sk-live-1234567890abcdef"));
    }

    #[test]
    fn test_sanitize_openai_key() {
        let message = "Using API key: sk-abcdefghijklmnopqrstuvwxyz1234567890ABCDEF";
        let sanitized = sanitize_log_message(message);
        assert!(sanitized.contains("***OPENAI_KEY_REDACTED***"));
    }

    #[test]
    fn test_sanitize_bot_token() {
        let message = "Bot token: 123456789:ABCdefGHIjklMNOpqrsTUVwxyz";
        let sanitized = sanitize_log_message(message);
        assert!(sanitized.contains("***BOT_TOKEN_REDACTED***"));
    }

    #[test]
    fn test_sanitize_connection_string() {
        let error = "Failed to connect: postgresql://admin:supersecret@localhost:5432/mydb";
        let sanitized = sanitize_error_message(error);
        assert!(sanitized.contains("***REDACTED***"));
        assert!(!sanitized.contains("supersecret"));
    }

    #[test]
    fn test_sanitized_string_display() {
        let sensitive = SanitizedString::new("token: secret123");
        let displayed = format!("{}", sensitive);
        assert!(displayed.contains("***REDACTED***"));
    }

    #[test]
    fn test_extension_trait() {
        let message = "Authorization: Bearer token123";
        let sanitized = message.sanitize_for_logs();
        assert!(sanitized.contains("***REDACTED***"));
    }
}
