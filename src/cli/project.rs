//! Project management subcommands (`jira project`).
//!
//! Provides commands to list accessible Jira projects and inspect project metadata.

use crate::client::JiraClient;
use crate::error::Result;
use crate::ui::format::OutputMode;
use crate::ui::table::render_projects_table;
use clap::{Args, Subcommand};
use colored::Colorize;

/// Project management commands.
#[derive(Debug, Clone, Args)]
pub struct ProjectArgs {
    /// Project operation to perform.
    #[command(subcommand)]
    pub command: ProjectSubcommand,
}

/// Available project subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum ProjectSubcommand {
    /// List accessible Jira projects.
    List,

    /// View details of a specific project.
    View(ProjectViewArgs),
}

/// Arguments for `jira project view <KEY>`.
#[derive(Debug, Clone, Args)]
pub struct ProjectViewArgs {
    /// Project key (e.g. DEV, PROJ).
    pub key: String,
}

/// Dispatches project subcommands.
pub async fn handle_project_command(
    args: ProjectArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    match args.command {
        ProjectSubcommand::List => handle_list(client, output_mode).await,
        ProjectSubcommand::View(view_args) => handle_view(view_args, client, output_mode).await,
    }
}

async fn handle_list(client: &JiraClient, output_mode: OutputMode) -> Result<()> {
    let projects = client.list_projects().await?;

    match output_mode {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&projects)?);
        }
        OutputMode::Markdown => {
            println!("| Key | Name | Type | Lead |");
            println!("| --- | --- | --- | --- |");
            for p in &projects {
                let lead = p
                    .lead
                    .as_ref()
                    .map(|u| u.display_name.as_str())
                    .unwrap_or("-");
                let ptype = p.project_type_key.as_deref().unwrap_or("-");
                println!("| {} | {} | {} | {} |", p.key, p.name, ptype, lead);
            }
        }
        OutputMode::Table | OutputMode::Plain => {
            println!("{}", render_projects_table(&projects));
        }
    }

    Ok(())
}

async fn handle_view(
    args: ProjectViewArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    let project = client.get_project(&args.key).await?;

    match output_mode {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&project)?);
        }
        OutputMode::Table | OutputMode::Plain | OutputMode::Markdown => {
            println!("\n{} - {}", project.key.bold().cyan(), project.name.bold());
            if let Some(desc) = &project.description {
                println!("Description: {}\n", desc);
            }
            if let Some(lead) = &project.lead {
                println!(
                    "Lead:        {} ({})",
                    lead.display_name.bold(),
                    lead.account_id.dimmed()
                );
            }
            if let Some(ptype) = &project.project_type_key {
                println!("Type:        {}", ptype);
            }
            if !project.issue_types.is_empty() {
                let types = project
                    .issue_types
                    .iter()
                    .map(|t| t.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("Issue Types: {}", types);
            }
            println!();
        }
    }

    Ok(())
}
