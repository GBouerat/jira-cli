//! Error types and result definitions for `jira-cli`.
//!
//! Provides a unified [`JiraError`] enumeration covering configuration issues,
//! network/HTTP failures, Jira API error responses, ADF parsing, and terminal I/O.

use thiserror::Error;

/// Result type alias for operations throughout `jira-cli`.
pub type Result<T> = std::result::Result<T, JiraError>;

/// Unified error enum representing all possible failure modes in `jira-cli`.
#[derive(Debug, Error)]
pub enum JiraError {
    /// An error occurred while reading, writing, or resolving configuration.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Missing required credentials (instance URL, user email, or API token).
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Network or HTTP transport error.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Jira Cloud REST API returned an error response (e.g., 400, 401, 403, 404, 500).
    #[error("Jira API error (status {status}): {message}")]
    Api {
        /// HTTP status code returned by the Jira REST API.
        status: reqwest::StatusCode,
        /// Extracted error message or details from Jira's error payload.
        message: String,
        /// Optional collection of field-specific error messages.
        errors: Option<std::collections::HashMap<String, String>>,
    },

    /// Jira Cloud REST API rate-limited the client (HTTP 429).
    #[error("Rate limit exceeded (HTTP 429). Retry after {retry_after_seconds} seconds")]
    RateLimited {
        /// Suggested retry interval in seconds from the `Retry-After` header.
        retry_after_seconds: u64,
    },

    /// Atlassian Document Format (ADF) parsing or serialization failed.
    #[error("ADF processing error: {0}")]
    Adf(String),

    /// Input/Output error when interacting with local files, stdin, or stdout.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// URL parsing error.
    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    /// An interactive prompt was aborted or encountered an error.
    #[error("Prompt error: {0}")]
    Prompt(String),

    /// A generic or internal error.
    #[error("{0}")]
    Generic(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_formatting() {
        let err = JiraError::Config("missing profile".to_string());
        assert_eq!(err.to_string(), "Configuration error: missing profile");

        let auth_err = JiraError::Auth("missing token".to_string());
        assert_eq!(auth_err.to_string(), "Authentication error: missing token");

        let rate_err = JiraError::RateLimited {
            retry_after_seconds: 30,
        };
        assert_eq!(
            rate_err.to_string(),
            "Rate limit exceeded (HTTP 429). Retry after 30 seconds"
        );
    }
}
