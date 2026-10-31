//! CLI hierarchy, argument specifications, and command dispatchers.
//!
//! Submodules organize options for issue workflows ([`issue`]), sprint/board operations
//! ([`sprint`]), project queries ([`project`]), configuration wizardry ([`config`]),
//! and shell completions ([`completions`]).

pub mod api;
pub mod args;
pub mod completions;
pub mod config;
pub mod issue;
pub mod project;
pub mod sprint;

pub use api::{handle_api_command, read_file_or_stdin};
pub use args::{Cli, Commands};
pub use completions::run_completions;
pub use config::handle_config_command;
pub use issue::handle_issue_command;
pub use project::handle_project_command;
pub use sprint::handle_sprint_command;
