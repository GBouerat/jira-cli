//! Configuration models and serializable structures.
//!
//! Defines the structure of the configuration file, profiles, and resolved credentials.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root structure of the `jira-cli` configuration file (`config.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Name of the active profile to use when no profile is explicitly specified.
    #[serde(default = "default_profile_name")]
    pub default_profile: String,

    /// Collection of named configuration profiles.
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

fn default_profile_name() -> String {
    "default".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_profile: default_profile_name(),
            profiles: HashMap::new(),
        }
    }
}

/// A specific user profile containing connection and authentication details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Profile {
    /// Base URL of the Jira Cloud instance (e.g., `https://company.atlassian.net`).
    pub instance_url: String,

    /// User email address associated with the Atlassian Cloud account.
    pub user_email: String,

    /// Optional API token. If omitted from the file, can be read from `JIRA_API_TOKEN`.
    pub api_token: Option<String>,

    /// Default project key to use when omitted from CLI commands (e.g., `PROJ`).
    pub default_project: Option<String>,
}

/// Fully resolved credentials and settings ready for client initialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfig {
    /// Validated base URL of the Jira instance without a trailing slash.
    pub instance_url: String,

    /// User email address for authentication.
    pub user_email: String,

    /// Jira Cloud REST API token.
    pub api_token: String,

    /// Default project key if configured.
    pub default_project: Option<String>,
}

/// Command-line overrides that take precedence over config file and environment variables.
#[derive(Debug, Clone, Default)]
pub struct ConfigOverrides {
    /// Explicit Jira instance URL override.
    pub instance_url: Option<String>,

    /// Explicit user email override.
    pub user_email: Option<String>,

    /// Explicit API token override.
    pub api_token: Option<String>,

    /// Explicit default project key override.
    pub default_project: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let mut config = Config::default();
        let profile = Profile {
            instance_url: "https://example.atlassian.net".to_string(),
            user_email: "dev@example.com".to_string(),
            api_token: Some("secret123".to_string()),
            default_project: Some("DEV".to_string()),
        };
        config.profiles.insert("default".to_string(), profile);

        let json_str = serde_json::to_string_pretty(&config).expect("failed to serialize json");
        assert!(json_str.contains("\"default_profile\": \"default\""));
        assert!(json_str.contains("\"instance_url\": \"https://example.atlassian.net\""));

        let deserialized: Config = serde_json::from_str(&json_str).expect("failed to deserialize json");
        assert_eq!(config, deserialized);
    }
}
