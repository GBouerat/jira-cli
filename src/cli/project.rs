//! Project management subcommands (`jira project`).
//!
//! Provides commands to list accessible Jira projects and inspect project metadata.

use crate::client::JiraClient;
use crate::error::Result;
use crate::ui::format::OutputMode;
use crate::ui::table::{render_components_table, render_projects_table, render_versions_table};
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

    /// List release versions for a project.
    Versions(ProjectVersionsArgs),

    /// Create a new release version for a project.
    CreateVersion(ProjectCreateVersionArgs),

    /// List subsystem components for a project.
    Components(ProjectComponentsArgs),
}

/// Arguments for `jira project view <KEY>`.
#[derive(Debug, Clone, Args)]
pub struct ProjectViewArgs {
    /// Project key (e.g. DEV, PROJ).
    pub key: String,
}

/// Arguments for `jira project versions <KEY>`.
#[derive(Debug, Clone, Args)]
pub struct ProjectVersionsArgs {
    /// Project key (e.g. DEV, PROJ).
    pub key: String,
}

/// Arguments for `jira project create-version <KEY>`.
#[derive(Debug, Clone, Args)]
pub struct ProjectCreateVersionArgs {
    /// Project key (e.g. DEV, PROJ).
    pub key: String,

    /// Name of the new release version (e.g. '1.0.0').
    #[arg(short, long)]
    pub name: String,

    /// Optional release version description.
    #[arg(short, long)]
    pub description: Option<String>,

    /// Target release date in YYYY-MM-DD format.
    #[arg(short, long)]
    pub release_date: Option<String>,
}

/// Arguments for `jira project components <KEY>`.
#[derive(Debug, Clone, Args)]
pub struct ProjectComponentsArgs {
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
        ProjectSubcommand::Versions(v_args) => handle_versions(v_args, client, output_mode).await,
        ProjectSubcommand::CreateVersion(cv_args) => {
            handle_create_version(cv_args, client, output_mode).await
        }
        ProjectSubcommand::Components(c_args) => {
            handle_components(c_args, client, output_mode).await
        }
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

async fn handle_versions(
    args: ProjectVersionsArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    let versions = client.list_project_versions(&args.key).await?;

    match output_mode {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&versions)?);
        }
        OutputMode::Markdown => {
            println!("| ID | Name | Released | Release Date | Archived | Description |");
            println!("| --- | --- | --- | --- | --- | --- |");
            for v in &versions {
                let rel = if v.released { "yes" } else { "no" };
                let date = v.release_date.as_deref().unwrap_or("-");
                let arch = if v.archived { "yes" } else { "no" };
                let desc = v.description.as_deref().unwrap_or("-");
                println!(
                    "| {} | {} | {} | {} | {} | {} |",
                    v.id, v.name, rel, date, arch, desc
                );
            }
        }
        OutputMode::Table | OutputMode::Plain => {
            println!("{}", render_versions_table(&versions));
        }
    }

    Ok(())
}

async fn handle_create_version(
    args: ProjectCreateVersionArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    let version = client
        .create_version(
            &args.key,
            &args.name,
            args.description.as_deref(),
            args.release_date.as_deref(),
        )
        .await?;

    match output_mode {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&version)?);
        }
        OutputMode::Table | OutputMode::Plain | OutputMode::Markdown => {
            println!(
                "{} Created version {} (ID: {}) for project {}.",
                "✓".green().bold(),
                version.name.bold().cyan(),
                version.id.dimmed(),
                args.key.to_uppercase().bold()
            );
        }
    }

    Ok(())
}

async fn handle_components(
    args: ProjectComponentsArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    let components = client.list_project_components(&args.key).await?;

    match output_mode {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&components)?);
        }
        OutputMode::Markdown => {
            println!("| ID | Name | Lead | Description |");
            println!("| --- | --- | --- | --- |");
            for c in &components {
                let lead = c
                    .lead
                    .as_ref()
                    .map(|u| u.display_name.as_str())
                    .unwrap_or("-");
                let desc = c.description.as_deref().unwrap_or("-");
                println!("| {} | {} | {} | {} |", c.id, c.name, lead, desc);
            }
        }
        OutputMode::Table | OutputMode::Plain => {
            println!("{}", render_components_table(&components));
        }
    }

    Ok(())
}
