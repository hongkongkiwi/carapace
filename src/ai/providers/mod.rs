//! AI Provider Implementations
//!
//! Concrete implementations of the AiProvider trait for various AI services.

pub mod openai;
pub mod anthropic;

use crate::ai::{AiError, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::time::Duration;

/// Build headers for API requests
pub fn build_headers(api_key: &str, organization: Option<&str>) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();

    let auth_value = format!("Bearer {}", api_key);
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&auth_value)
            .map_err(|e| AiError::InvalidRequest(format!("Invalid API key: {}", e)))?,
    );

    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    if let Some(org) = organization {
        headers.insert(
            "OpenAI-Organization",
            HeaderValue::from_str(org)
                .map_err(|e| AiError::InvalidRequest(format!("Invalid organization: {}", e)))?,
        );
    }

    Ok(headers)
}

/// Build a reqwest client with timeout and retry settings
pub fn build_client(timeout_seconds: u64) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_seconds))
        .build()
        .map_err(|e| AiError::NetworkError(e))
}

/// Handle API errors with proper status code mapping
pub async fn handle_api_error(response: reqwest::Response) -> Result<reqwest::Response> {
    let status = response.status();

    if status.is_success() {
        return Ok(response);
    }

    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "Unknown error".to_string());

    match status.as_u16() {
        401 => Err(AiError::AuthenticationError(body)),
        429 => {
            // Try to extract retry-after header
            Err(AiError::RateLimitExceeded { retry_after: 60 })
        }
        400 | 422 => Err(AiError::InvalidRequest(body)),
        _ => Err(AiError::ApiError(format!("{}: {}", status, body))),
    }
}

/// Parse SSE (Server-Sent Events) stream data
pub fn parse_sse_line(line: &str) -> Option<&str> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    if let Some(data) = line.strip_prefix("data: ") {
        if data == "[DONE]" {
            return None;
        }
        return Some(data);
    }

    None
}
