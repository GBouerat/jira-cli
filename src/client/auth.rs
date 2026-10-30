//! Authentication header generation and client identity metadata.
//!
//! Jira Cloud REST API v3 uses HTTP Basic authentication with an Atlassian
//! account email and API token: `Authorization: Basic <base64(email:token)>`.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;

/// Builds the `Authorization: Basic ...` header value for Jira Cloud REST API.
///
/// # Arguments
/// * `email` - User email address.
/// * `token` - Jira API token.
pub fn build_basic_auth_header(email: &str, token: &str) -> String {
    let credentials = format!("{}:{}", email.trim(), token.trim());
    let encoded = BASE64.encode(credentials.as_bytes());
    format!("Basic {}", encoded)
}

/// Builds the standard `User-Agent` header containing crate name, version, and repository URL.
pub fn build_user_agent_header() -> String {
    format!(
        "{}/{} ({})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_REPOSITORY")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_basic_auth_header() {
        let header = build_basic_auth_header("user@example.com", "my-secret-token");
        assert!(header.starts_with("Basic "));
        // "user@example.com:my-secret-token" encoded
        let decoded = BASE64
            .decode(&header["Basic ".len()..])
            .expect("valid base64");
        assert_eq!(
            String::from_utf8(decoded).unwrap(),
            "user@example.com:my-secret-token"
        );
    }

    #[test]
    fn test_build_user_agent_header() {
        let ua = build_user_agent_header();
        assert!(ua.contains("jira-cli"));
        assert!(ua.contains("https://github.com/GBouerat/jira-cli"));
    }
}
