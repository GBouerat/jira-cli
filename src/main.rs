//! Entry point for the `jira` command-line executable.
//!
//! Handles CLI argument parsing, configuration loading, error formatting,
//! and command dispatching.

use clap::{CommandFactory, Parser};
use colored::Colorize;
use jira_cli::cli::args::{Cli, Commands};
use jira_cli::cli::{
    handle_config_command, handle_issue_command, handle_project_command, handle_sprint_command, run_completions,
};
use jira_cli::client::JiraClient;
use jira_cli::config::{load_config, resolve_credentials};
use jira_cli::error::Result;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Err(err) = run(cli).await {
        eprintln!("{} {}", "Error:".red().bold(), err);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

async fn run(cli: Cli) -> Result<()> {
    // 1. Completions subcommand requires neither configuration nor network
    if let Commands::Completions(comp_args) = cli.command {
        let mut cmd = Cli::command();
        run_completions(comp_args, &mut cmd);
        return Ok(());
    }

    let config_path = cli.config.clone();

    // 2. Config subcommands manage profiles and don't necessarily require existing valid credentials
    let overrides = cli.to_overrides();
    if let Commands::Config(cfg_args) = cli.command {
        let profile_opt = cli.profile.clone();
        let cfg_path_clone = config_path.clone();

        return handle_config_command(cfg_args, config_path.as_ref(), move || {
            let loaded = load_config(cfg_path_clone.as_deref())?;
            let resolved = resolve_credentials(&loaded, profile_opt.as_deref(), overrides)?;
            JiraClient::new(&resolved)
        })
        .await;
    }

    // 3. All other operations require resolved Jira credentials
    let loaded_config = load_config(config_path.as_deref())?;
    let resolved = resolve_credentials(&loaded_config, cli.profile.as_deref(), cli.to_overrides())?;
    let client = JiraClient::new(&resolved)?;
    let default_project = resolved.default_project.as_deref();

    match cli.command {
        Commands::Issue(issue_args) => {
            handle_issue_command(issue_args, &client, cli.output, default_project).await?;
        }
        Commands::Sprint(sprint_args) => {
            handle_sprint_command(sprint_args, &client, cli.output).await?;
        }
        Commands::Project(project_args) => {
            handle_project_command(project_args, &client, cli.output).await?;
        }
        Commands::Config(_) | Commands::Completions(_) => unreachable!(),
    }

    Ok(())
}
