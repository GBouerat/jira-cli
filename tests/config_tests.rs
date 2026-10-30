//! Integration tests for configuration resolution precedence and filesystem permissions.

mod common;

use common::{EnvGuard, ENV_LOCK};
use jira_cli::config::model::{Config, ConfigOverrides, Profile};
use jira_cli::config::resolver::{resolve_credentials, save_config};
use tempfile::tempdir;

#[test]
fn test_config_resolution_precedence() {
    let _lock = ENV_LOCK.lock().unwrap();

    let mut config = Config::default();
    config.profiles.insert(
        "default".to_string(),
        Profile {
            instance_url: "https://file.atlassian.net".to_string(),
            user_email: "file@example.com".to_string(),
            api_token: Some("file-token".to_string()),
            default_project: Some("FILE".to_string()),
        },
    );

    // 1. Baseline: loaded directly from config
    let resolved = resolve_credentials(&config, None, ConfigOverrides::default()).unwrap();
    assert_eq!(resolved.instance_url, "https://file.atlassian.net");
    assert_eq!(resolved.user_email, "file@example.com");
    assert_eq!(resolved.api_token, "file-token");

    // 2. Environment variables override config file
    {
        let _env = EnvGuard::set_many(&[
            ("JIRA_INSTANCE_URL", "https://env.atlassian.net"),
            ("JIRA_USER_EMAIL", "env@example.com"),
            ("JIRA_API_TOKEN", "env-token"),
        ]);

        let resolved_env = resolve_credentials(&config, None, ConfigOverrides::default()).unwrap();
        assert_eq!(resolved_env.instance_url, "https://env.atlassian.net");
        assert_eq!(resolved_env.user_email, "env@example.com");
        assert_eq!(resolved_env.api_token, "env-token");

        // 3. CLI arguments override environment variables
        let overrides = ConfigOverrides {
            instance_url: Some("https://cli.atlassian.net".to_string()),
            user_email: Some("cli@example.com".to_string()),
            api_token: Some("cli-token".to_string()),
            default_project: Some("CLI".to_string()),
        };

        let resolved_cli = resolve_credentials(&config, None, overrides).unwrap();
        assert_eq!(resolved_cli.instance_url, "https://cli.atlassian.net");
        assert_eq!(resolved_cli.user_email, "cli@example.com");
        assert_eq!(resolved_cli.api_token, "cli-token");
        assert_eq!(resolved_cli.default_project.as_deref(), Some("CLI"));
    }
}

#[cfg(unix)]
#[test]
fn test_config_file_permissions_unix() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().unwrap();
    let path = dir.path().join("secure_config.json");

    let config = Config::default();
    save_config(&config, Some(&path)).unwrap();

    let metadata = std::fs::metadata(&path).unwrap();
    let mode = metadata.permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "Config file permissions must be 0600 (read/write only for owner)");
}
