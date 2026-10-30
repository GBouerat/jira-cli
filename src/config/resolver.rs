//! Configuration resolver, filesystem loading/saving, and credential resolution.
//!
//! Enforces configuration precedence: CLI flags > Environment variables > Configuration file.

use crate::config::model::{Config, ConfigOverrides, ResolvedConfig};
use crate::error::{JiraError, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Name of the primary configuration file.
pub const CONFIG_FILE_NAME: &str = "config.json";

/// Name of the configuration directory inside the user's config directory.
pub const CONFIG_DIR_NAME: &str = "jira-cli";

/// Resolves the default path to the configuration file (`~/.config/jira-cli/config.json`).
///
/// # Errors
/// Returns [`JiraError::Config`] if the user's config or home directory cannot be determined.
pub fn default_config_path() -> Result<PathBuf> {
    let base = dirs::config_dir().or_else(dirs::home_dir).ok_or_else(|| {
        JiraError::Config("Unable to determine user home or config directory".to_string())
    })?;
    Ok(base.join(CONFIG_DIR_NAME).join(CONFIG_FILE_NAME))
}

/// Normalizes a Jira instance URL by stripping trailing slashes and ensuring an `https://` scheme.
///
/// # Arguments
/// * `raw_url` - The raw URL string entered by the user.
///
/// # Errors
/// Returns [`JiraError::Config`] if the URL is empty or malformed.
pub fn normalize_instance_url(raw_url: &str) -> Result<String> {
    let trimmed = raw_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(JiraError::Config(
            "Jira instance URL cannot be empty".to_string(),
        ));
    }

    let url_with_scheme = if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        format!("https://{}", trimmed)
    } else {
        trimmed.to_string()
    };

    let parsed = url::Url::parse(&url_with_scheme).map_err(|e| {
        JiraError::Config(format!("Invalid Jira instance URL '{}': {}", raw_url, e))
    })?;

    if parsed.host_str().is_none() {
        return Err(JiraError::Config(format!(
            "Invalid Jira instance URL '{}': missing host",
            raw_url
        )));
    }

    Ok(url_with_scheme)
}

/// Loads the configuration file from disk.
///
/// If the file does not exist, an empty default [`Config`] is returned.
///
/// # Arguments
/// * `path` - Optional custom path to the configuration file.
///
/// # Errors
/// Returns [`JiraError`] on I/O or JSON deserialization errors.
pub fn load_config(path: Option<&Path>) -> Result<Config> {
    let target_path = match path {
        Some(p) => p.to_path_buf(),
        None => default_config_path()?,
    };

    if !target_path.exists() {
        return Ok(Config::default());
    }

    let contents = fs::read_to_string(&target_path).map_err(JiraError::Io)?;
    let config: Config = serde_json::from_str(&contents).map_err(|e| {
        JiraError::Config(format!(
            "Failed to parse config file at {:?}: {}",
            target_path, e
        ))
    })?;

    Ok(config)
}

/// Saves the configuration file to disk, ensuring secure file permissions (`0600` on Unix).
///
/// # Arguments
/// * `config` - The configuration to serialize.
/// * `path` - Optional custom path to save to.
///
/// # Errors
/// Returns [`JiraError`] on serialization or filesystem write errors.
pub fn save_config(config: &Config, path: Option<&Path>) -> Result<PathBuf> {
    let target_path = match path {
        Some(p) => p.to_path_buf(),
        None => default_config_path()?,
    };

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(JiraError::Io)?;
    }

    let serialized = serde_json::to_string_pretty(config).map_err(|e| {
        JiraError::Config(format!("Failed to serialize configuration to JSON: {}", e))
    })?;

    fs::write(&target_path, serialized).map_err(JiraError::Io)?;

    #[cfg(unix)]
    {
        let permissions = fs::Permissions::from_mode(0o600);
        let _ = fs::set_permissions(&target_path, permissions);
    }

    Ok(target_path)
}

/// Resolves complete authentication credentials and settings following precedence rules:
/// 1. Command-line overrides (`ConfigOverrides`)
/// 2. Environment variables (`JIRA_INSTANCE_URL`, `JIRA_USER_EMAIL`, `JIRA_API_TOKEN`, etc.)
/// 3. Configuration file profile
///
/// # Arguments
/// * `config` - The loaded configuration structure.
/// * `profile_name` - Optional specific profile name to load.
/// * `overrides` - Explicit command-line overrides.
///
/// # Errors
/// Returns [`JiraError::Auth`] if instance URL, user email, or API token cannot be resolved.
pub fn resolve_credentials(
    config: &Config,
    profile_name: Option<&str>,
    overrides: ConfigOverrides,
) -> Result<ResolvedConfig> {
    // Determine profile to use: CLI flag > JIRA_PROFILE env var > config.default_profile
    let selected_profile_name = profile_name
        .map(|s| s.to_string())
        .or_else(|| std::env::var("JIRA_PROFILE").ok())
        .unwrap_or_else(|| config.default_profile.clone());

    let profile = config
        .profiles
        .get(&selected_profile_name)
        .cloned()
        .unwrap_or_default();

    // 1. Instance URL resolution
    let instance_url_raw = overrides
        .instance_url
        .or_else(|| std::env::var("JIRA_INSTANCE_URL").ok())
        .or_else(|| {
            if profile.instance_url.is_empty() {
                None
            } else {
                Some(profile.instance_url.clone())
            }
        })
        .ok_or_else(|| {
            JiraError::Auth(
                "Jira instance URL not found. Set via --instance, JIRA_INSTANCE_URL env var, or 'jira config init'"
                    .to_string(),
            )
        })?;

    let instance_url = normalize_instance_url(&instance_url_raw)?;

    // 2. User email resolution
    let user_email = overrides
        .user_email
        .or_else(|| std::env::var("JIRA_USER_EMAIL").ok())
        .or_else(|| {
            if profile.user_email.is_empty() {
                None
            } else {
                Some(profile.user_email.clone())
            }
        })
        .ok_or_else(|| {
            JiraError::Auth(
                "Jira user email not found. Set via --email, JIRA_USER_EMAIL env var, or 'jira config init'"
                    .to_string(),
            )
        })?;

    // 3. API token resolution
    let api_token = overrides
        .api_token
        .or_else(|| std::env::var("JIRA_API_TOKEN").ok())
        .or_else(|| profile.api_token.clone())
        .ok_or_else(|| {
            JiraError::Auth(
                "Jira API token not found. Set via --token, JIRA_API_TOKEN env var, or 'jira config init'"
                    .to_string(),
            )
        })?;

    // 4. Default project resolution
    let default_project = overrides
        .default_project
        .or_else(|| std::env::var("JIRA_DEFAULT_PROJECT").ok())
        .or_else(|| profile.default_project.clone());

    Ok(ResolvedConfig {
        instance_url,
        user_email,
        api_token,
        default_project,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::Profile;
    use tempfile::tempdir;

    #[test]
    fn test_normalize_instance_url() {
        assert_eq!(
            normalize_instance_url("mycompany.atlassian.net").unwrap(),
            "https://mycompany.atlassian.net"
        );
        assert_eq!(
            normalize_instance_url("https://mycompany.atlassian.net/").unwrap(),
            "https://mycompany.atlassian.net"
        );
        assert_eq!(
            normalize_instance_url("http://localhost:8080/").unwrap(),
            "http://localhost:8080"
        );
        assert!(normalize_instance_url("   ").is_err());
    }

    #[test]
    fn test_save_and_load_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");

        let mut config = Config::default();
        config.profiles.insert(
            "work".to_string(),
            Profile {
                instance_url: "https://work.atlassian.net".to_string(),
                user_email: "work@example.com".to_string(),
                api_token: Some("token123".to_string()),
                default_project: Some("WORK".to_string()),
            },
        );
        config.default_profile = "work".to_string();

        save_config(&config, Some(&config_path)).unwrap();

        let loaded = load_config(Some(&config_path)).unwrap();
        assert_eq!(loaded.default_profile, "work");
        assert_eq!(
            loaded.profiles.get("work").unwrap().instance_url,
            "https://work.atlassian.net"
        );
    }
}
