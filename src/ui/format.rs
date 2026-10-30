//! Output mode definition and parsing.
//!
//! Controls whether command results are displayed as formatted terminal tables,
//! raw JSON for Unix piping, Markdown, or plain text.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Supported output formatting modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputMode {
    /// Formatted UTF-8 terminal table or colorized detail view.
    #[default]
    Table,

    /// Machine-readable JSON output for piping into `jq` or external tools.
    Json,

    /// Markdown representation suitable for notes or copy-pasting into docs.
    Markdown,

    /// Plain unadorned text output.
    Plain,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_mode_defaults() {
        assert_eq!(OutputMode::default(), OutputMode::Table);
    }
}
