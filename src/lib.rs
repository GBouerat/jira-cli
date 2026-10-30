//! `jira-cli` - A modern, developer-friendly Jira Cloud CLI built in Rust using Jira REST API v3.
//!
//! # Architecture Overview
//!
//! `jira-cli` is organized into modular components:
//! - [`adf`]: Atlassian Document Format (ADF) parser, serializer, and terminal renderer.
//! - [`client`]: Async Jira Cloud REST API v3 client with rate-limiting backoff and error handling.
//! - [`config`]: Configuration loading, profile persistence, and credential precedence resolution.
//! - [`ui`]: Terminal presentation with dynamic tables and colorized detail views.
//! - [`cli`]: Command-line interface definitions and subcommand dispatchers.
//! - [`error`]: Comprehensive error types and result definitions.
//!
//! GitHub Repository: <https://github.com/GBouerat/jira-cli>

pub mod adf;
pub mod cli;
pub mod client;
pub mod config;
pub mod error;
pub mod ui;

pub use client::JiraClient;
pub use config::{Config, Profile, ResolvedConfig};
pub use error::{JiraError, Result};
