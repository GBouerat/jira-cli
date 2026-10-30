//! Top-level CLI argument specification and global flags.
//!
//! Configures `--version` and `--help` to feature GitHub repository attribution,
//! defines global execution options (profiles, formatters, overrides), and declares
//! all primary subcommands.

use crate::cli::completions::CompletionsArgs;
use crate::cli::config::ConfigArgs;
use crate::cli::issue::IssueArgs;
use crate::cli::project::ProjectArgs;
use crate::cli::sprint::SprintArgs;
use crate::config::ConfigOverrides;
use crate::ui::format::OutputMode;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Top-level command-line interface specification for `jira`.
#[derive(Debug, Parser)]
#[command(
    name = "jira",
    author = "Guillaume Bouérat",
    version,
    long_version = concat!(
        env!("CARGO_PKG_VERSION"),
        "\nhttps://github.com/GBouerat/jira-cli"
    ),
    about = "A modern, developer-friendly Jira Cloud CLI built in Rust using Jira REST API v3",
    after_help = "GitHub Repository & Documentation:\n  https://github.com/GBouerat/jira-cli"
)]
pub struct Cli {
    /// Output formatting mode.
    #[arg(short, long, value_enum, default_value_t = OutputMode::Table, global = true)]
    pub output: OutputMode,

    /// Configuration profile to use.
    #[arg(short, long, global = true)]
    pub profile: Option<String>,

    /// Path to a custom configuration file.
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Explicit Jira instance URL override.
    #[arg(long, global = true)]
    pub instance: Option<String>,

    /// Explicit user email override.
    #[arg(long, global = true)]
    pub email: Option<String>,

    /// Explicit API token override (note: visible in process table; prefer JIRA_API_TOKEN env var).
    #[arg(long, global = true)]
    pub token: Option<String>,

    /// Enable verbose debug output.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    /// Extracts explicit CLI overrides into [`ConfigOverrides`].
    pub fn to_overrides(&self) -> ConfigOverrides {
        ConfigOverrides {
            instance_url: self.instance.clone(),
            user_email: self.email.clone(),
            api_token: self.token.clone(),
            default_project: None,
        }
    }
}

/// Available subcommands for the `jira` binary.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Issue operations (list, view, create, edit, assign, transition, comment, worklog, open).
    Issue(IssueArgs),

    /// Agile sprint and board operations.
    Sprint(SprintArgs),

    /// Project queries and metadata.
    Project(ProjectArgs),

    /// Configuration profile management, initialization, and connectivity testing.
    Config(ConfigArgs),

    /// Generate shell completions (bash, zsh).
    Completions(CompletionsArgs),
}
