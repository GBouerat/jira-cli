//! Integration tests for CLI arguments, help, version, and shell completions.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_version_metadata() {
    let mut cmd = Command::cargo_bin("jira").unwrap();
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")))
        .stdout(predicate::str::contains(
            "https://github.com/GBouerat/jira-cli",
        ));
}

#[test]
fn test_cli_help_metadata() {
    let mut cmd = Command::cargo_bin("jira").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("jira"))
        .stdout(predicate::str::contains(
            "https://github.com/GBouerat/jira-cli",
        ));
}

#[test]
fn test_cli_completions_bash() {
    let mut cmd = Command::cargo_bin("jira").unwrap();
    cmd.args(["completions", "bash"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("complete -F _jira"));
}

#[test]
fn test_cli_completions_zsh() {
    let mut cmd = Command::cargo_bin("jira").unwrap();
    cmd.args(["completions", "zsh"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("#compdef jira"));
}

#[test]
fn test_cli_missing_credentials_fails_gracefully() {
    let mut cmd = Command::cargo_bin("jira").unwrap();
    cmd.env_remove("JIRA_INSTANCE_URL")
        .env_remove("JIRA_USER_EMAIL")
        .env_remove("JIRA_API_TOKEN")
        .env_remove("JIRA_PROFILE")
        .args(["--config", "/nonexistent/path/config.json", "issue", "list"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Authentication error"));
}

#[test]
fn test_cli_subcommands_help_metadata() {
    // 1. comment-get help
    let mut cmd = Command::cargo_bin("jira").unwrap();
    cmd.args(["issue", "comment-get", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("comment-get"))
        .stdout(predicate::str::contains("<KEY>"))
        .stdout(predicate::str::contains("<COMMENT_ID>"));

    // 2. comment-edit help
    let mut cmd = Command::cargo_bin("jira").unwrap();
    cmd.args(["issue", "comment-edit", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--adf"))
        .stdout(predicate::str::contains("--body-file"))
        .stdout(predicate::str::contains("--dry-run"));

    // 3. comment-delete help
    let mut cmd = Command::cargo_bin("jira").unwrap();
    cmd.args(["issue", "comment-delete", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("-y"))
        .stdout(predicate::str::contains("<COMMENT_ID>"));

    // 4. create-subtasks help
    let mut cmd = Command::cargo_bin("jira").unwrap();
    cmd.args(["issue", "create-subtasks", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("<PARENT>"))
        .stdout(predicate::str::contains("<FILE>"))
        .stdout(predicate::str::contains("--limit"));

    // 5. set-priority help
    let mut cmd = Command::cargo_bin("jira").unwrap();
    cmd.args(["issue", "set-priority", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("<PRIORITY>"))
        .stdout(predicate::str::contains("<KEYS>..."));

    // 6. api help
    let mut cmd = Command::cargo_bin("jira").unwrap();
    cmd.args(["api", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("<METHOD>"))
        .stdout(predicate::str::contains("<PATH>"))
        .stdout(predicate::str::contains("--body-file"));
}
