//! Configuration management and credential resolution for `jira-cli`.
//!
//! Submodules provide data structures for profiles, filesystem loading/saving,
//! and resolution precedence across CLI arguments, environment variables, and config files.

pub mod model;
pub mod resolver;

pub use model::{Config, ConfigOverrides, Profile, ResolvedConfig};
pub use resolver::{default_config_path, load_config, normalize_instance_url, resolve_credentials, save_config};
