//! Configuration management subcommands.
//!
//! Provides the interactive configuration wizard (`jira config init`), profile
//! listing (`jira config list`), profile switching (`jira config use`), and
//! connectivity/credentials testing (`jira config test`).

use crate::client::JiraClient;
use crate::config::model::Profile;
use crate::config::resolver::{
    default_config_path, load_config, normalize_instance_url, save_config,
};
use crate::error::{JiraError, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

/// Commands for viewing and modifying `jira-cli` configuration and profiles.
#[derive(Debug, Clone, Args)]
pub struct ConfigArgs {
    /// Configuration action to execute.
    #[command(subcommand)]
    pub command: ConfigSubcommand,
}

/// Available configuration subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum ConfigSubcommand {
    /// Run setup wizard or configure credentials for a profile.
    Init(ConfigInitArgs),

    /// List all configured profiles and show the active profile.
    List,

    /// Switch the default active profile.
    Use(ConfigUseArgs),

    /// Verify connectivity and authentication against Jira Cloud.
    Test,
}

/// Arguments for `jira config init`.
#[derive(Debug, Clone, Args)]
pub struct ConfigInitArgs {
    /// Name of profile to create or update (defaults to 'default').
    #[arg(short, long, default_value = "default")]
    pub profile: String,

    /// Jira Cloud instance URL (e.g., <https://my-team.atlassian.net>).
    #[arg(long)]
    pub instance: Option<String>,

    /// Atlassian account email address.
    #[arg(long)]
    pub email: Option<String>,

    /// Atlassian API token.
    #[arg(long)]
    pub token: Option<String>,

    /// Default Jira project key (e.g., DEV).
    #[arg(long)]
    pub project: Option<String>,

    /// Non-interactive mode (requires instance, email, and token flags).
    #[arg(long)]
    pub non_interactive: bool,
}

/// Arguments for `jira config use <NAME>`.
#[derive(Debug, Clone, Args)]
pub struct ConfigUseArgs {
    /// Name of the profile to set as active default.
    pub name: String,
}

/// Executes the config command suite.
pub async fn handle_config_command(
    args: ConfigArgs,
    config_path: Option<&PathBuf>,
    client_factory: impl FnOnce() -> Result<JiraClient>,
) -> Result<()> {
    match args.command {
        ConfigSubcommand::Init(init_args) => handle_init(init_args, config_path),
        ConfigSubcommand::List => handle_list(config_path),
        ConfigSubcommand::Use(use_args) => handle_use(use_args, config_path),
        ConfigSubcommand::Test => handle_test(client_factory).await,
    }
}

fn handle_init(args: ConfigInitArgs, config_path: Option<&PathBuf>) -> Result<()> {
    let mut config = load_config(config_path.map(|p| p.as_path()))?;

    let (instance_url, email, token, project) = if args.non_interactive {
        let instance = args.instance.ok_or_else(|| {
            JiraError::Config("--instance is required in non-interactive mode".to_string())
        })?;
        let email = args.email.ok_or_else(|| {
            JiraError::Config("--email is required in non-interactive mode".to_string())
        })?;
        let token = args.token.ok_or_else(|| {
            JiraError::Config("--token is required in non-interactive mode".to_string())
        })?;
        (instance, email, token, args.project)
    } else {
        println!("{}", "─── Jira CLI Configuration Wizard ───".bold().cyan());
        println!("Configure your Atlassian Cloud credentials and instance settings.\n");

        let instance_input: String = match args.instance {
            Some(i) => i,
            None => inquire::Text::new("Jira instance URL (e.g. https://company.atlassian.net):")
                .with_placeholder("https://company.atlassian.net")
                .prompt()
                .map_err(|e| JiraError::Prompt(e.to_string()))?,
        };

        let email_input: String = match args.email {
            Some(e) => e,
            None => inquire::Text::new("Atlassian account email:")
                .prompt()
                .map_err(|e| JiraError::Prompt(e.to_string()))?,
        };

        let token_input: String = match args.token {
            Some(t) => t,
            None => inquire::Password::new("Atlassian API token (create at https://id.atlassian.com/manage-profile/security/api-tokens):")
                .without_confirmation()
                .prompt()
                .map_err(|e| JiraError::Prompt(e.to_string()))?,
        };

        let project_input: Option<String> = match args.project {
            Some(p) => Some(p),
            None => {
                let p = inquire::Text::new("Default project key (optional, e.g. DEV):")
                    .prompt()
                    .map_err(|e| JiraError::Prompt(e.to_string()))?;
                let trimmed = p.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_uppercase())
                }
            }
        };

        (instance_input, email_input, token_input, project_input)
    };

    let normalized_url = normalize_instance_url(&instance_url)?;

    let profile = Profile {
        instance_url: normalized_url,
        user_email: email.trim().to_string(),
        api_token: Some(token.trim().to_string()),
        default_project: project.map(|p| p.trim().to_uppercase()),
    };

    let profile_name = args.profile.trim();
    config.profiles.insert(profile_name.to_string(), profile);

    if config.default_profile.is_empty() || config.profiles.len() == 1 {
        config.default_profile = profile_name.to_string();
    }

    let saved_path = save_config(&config, config_path.map(|p| p.as_path()))?;
    println!(
        "\n{} Configuration saved for profile '{}' to {}",
        "✔".green().bold(),
        profile_name.cyan(),
        saved_path.display().to_string().dimmed()
    );
    println!("Test connectivity with: {}", "jira config test".yellow());

    Ok(())
}

fn handle_list(config_path: Option<&PathBuf>) -> Result<()> {
    let config = load_config(config_path.map(|p| p.as_path()))?;
    let path = config_path
        .cloned()
        .unwrap_or_else(|| default_config_path().unwrap_or_default());

    println!("Config file: {}\n", path.display().to_string().dimmed());

    if config.profiles.is_empty() {
        println!(
            "No profiles configured yet. Run {} to get started.",
            "jira config init".yellow()
        );
        return Ok(());
    }

    println!("{}", "Profiles:".bold());
    for (name, prof) in &config.profiles {
        let is_default = name == &config.default_profile;
        let prefix = if is_default {
            format!("* {}", name).green().bold()
        } else {
            format!("  {}", name).normal()
        };

        let proj = prof
            .default_project
            .as_deref()
            .map(|p| format!(" (project: {})", p))
            .unwrap_or_default();

        println!("{} -> {}{}", prefix, prof.instance_url, proj);
        println!("    user: {}", prof.user_email.dimmed());
    }

    Ok(())
}

fn handle_use(args: ConfigUseArgs, config_path: Option<&PathBuf>) -> Result<()> {
    let mut config = load_config(config_path.map(|p| p.as_path()))?;
    let target = args.name.trim();

    if !config.profiles.contains_key(target) {
        return Err(JiraError::Config(format!(
            "Profile '{}' does not exist. Available profiles: {}",
            target,
            config
                .profiles
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    config.default_profile = target.to_string();
    save_config(&config, config_path.map(|p| p.as_path()))?;

    println!(
        "{} Active profile set to '{}'",
        "✔".green().bold(),
        target.cyan()
    );
    Ok(())
}

async fn handle_test(client_factory: impl FnOnce() -> Result<JiraClient>) -> Result<()> {
    let client = client_factory()?;
    println!("Testing connection to {}...", client.instance_url().cyan());

    let user = client.myself().await?;
    println!("\n{} Authentication successful!", "✔".green().bold());
    println!(
        "  User:        {} ({})",
        user.display_name.bold(),
        user.account_id.dimmed()
    );
    if let Some(email) = &user.email_address {
        println!("  Email:       {}", email);
    }
    println!("  Instance:    {}", client.instance_url());
    println!(
        "  Status:      {}",
        if user.active {
            "Active".green()
        } else {
            "Inactive".red()
        }
    );

    Ok(())
}
