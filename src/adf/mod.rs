//! Atlassian Document Format (ADF) parsing, AST structures, and terminal rendering.
//!
//! Jira Cloud REST API v3 requires ADF for rich-text fields such as issue descriptions,
//! comments, and worklog notes. This module provides complete AST models ([`ast`]),
//! conversion from Markdown text to ADF ([`from_text`]), and rendering of ADF
//! trees into terminal-friendly text ([`to_terminal`]).

pub mod ast;
pub mod from_text;
pub mod to_terminal;

pub use ast::{DocNode, Mark, Node};
pub use from_text::{parse_inline, text_to_adf};
pub use to_terminal::adf_to_terminal;
