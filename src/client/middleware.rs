//! Response error handling, rate limiting detection, and HTTP error translation.

use crate::error::{JiraError, Result};
use reqwest::{Response, StatusCode};
use serde::Deserialize;
use std::collections::HashMap;

/// Standard error representation returned by Jira Cloud REST API v3.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct JiraErrorPayload {
    /// List of top-level error messages.
    #[serde(default)]
    pub error_messages: Vec<String>,

    /// Map of specific field names to error messages.
    #[serde(default)]
    pub errors: HashMap<String, String>,
}

/// Inspects an HTTP response and converts error status codes into [`JiraError`].
///
/// If the response is successful (2xx), returns the response unmodified.
///
/// # Arguments
/// * `response` - The HTTP response received from Jira.
///
/// # Errors
/// * Returns [`JiraError::RateLimited`] if status is HTTP 429.
/// * Returns [`JiraError::Api`] for other 4xx/5xx responses with parsed error payloads.
pub async fn handle_response_errors(response: Response) -> Result<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    // Handle HTTP 429 Too Many Requests
    if status == StatusCode::TOO_MANY_REQUESTS {
        let retry_after = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(5); // Default to 5 seconds if unspecified

        return Err(JiraError::RateLimited {
            retry_after_seconds: retry_after,
        });
    }

    // Attempt to extract structured Jira error messages from the JSON body
    let body_text = response.text().await.unwrap_or_default();
    let parsed_payload: Option<JiraErrorPayload> = serde_json::from_str(&body_text).ok();

    let (message, errors) = match parsed_payload {
        Some(payload) => {
            let mut msgs = payload.error_messages;
            if msgs.is_empty() && !payload.errors.is_empty() {
                for (field, msg) in &payload.errors {
                    msgs.push(format!("{}: {}", field, msg));
                }
            }
            let combined = if msgs.is_empty() {
                body_text.clone()
            } else {
                msgs.join("; ")
            };
            let errors_map = if payload.errors.is_empty() {
                None
            } else {
                Some(payload.errors)
            };
            (combined, errors_map)
        }
        None => {
            let fallback_msg = if body_text.trim().is_empty() {
                format!("HTTP error {}", status)
            } else {
                body_text
            };
            (fallback_msg, None)
        }
    };

    Err(JiraError::Api {
        status,
        message,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_jira_error_payload() {
        let json = r#"{
            "errorMessages": ["Issue does not exist or you do not have permission to view it."],
            "errors": {
                "summary": "Field is required"
            }
        }"#;

        let payload: JiraErrorPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.error_messages.len(), 1);
        assert_eq!(
            payload.errors.get("summary"),
            Some(&"Field is required".to_string())
        );
    }
}
