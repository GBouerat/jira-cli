//! Sprint and Agile board subcommands (`jira sprint`).
//!
//! Provides commands to list Agile boards, list sprints, and view issues assigned to a sprint.

use crate::client::JiraClient;
use crate::error::Result;
use crate::ui::format::OutputMode;
use crate::ui::table::{render_boards_table, render_issues_table, render_sprints_table};
use clap::{Args, Subcommand};

/// Sprint and Agile board commands.
#[derive(Debug, Clone, Args)]
pub struct SprintArgs {
    /// Sprint operation to perform.
    #[command(subcommand)]
    pub command: SprintSubcommand,
}

/// Available sprint subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum SprintSubcommand {
    /// List sprints for an Agile board.
    List(SprintListArgs),

    /// List issues within a specific sprint.
    Issues(SprintIssuesArgs),

    /// List Agile Scrum/Kanban boards.
    Boards,
}

/// Arguments for `jira sprint list`.
#[derive(Debug, Clone, Args)]
pub struct SprintListArgs {
    /// Agile board ID (run 'jira sprint boards' to find IDs).
    #[arg(short, long)]
    pub board: u64,

    /// Filter by sprint state: 'active', 'future', or 'closed'.
    #[arg(short, long)]
    pub state: Option<String>,
}

/// Arguments for `jira sprint issues <ID>`.
#[derive(Debug, Clone, Args)]
pub struct SprintIssuesArgs {
    /// Sprint ID.
    pub id: u64,
}

/// Dispatches sprint subcommands.
pub async fn handle_sprint_command(
    args: SprintArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    match args.command {
        SprintSubcommand::List(list_args) => handle_list(list_args, client, output_mode).await,
        SprintSubcommand::Issues(issues_args) => handle_issues(issues_args, client, output_mode).await,
        SprintSubcommand::Boards => handle_boards(client, output_mode).await,
    }
}

async fn handle_list(args: SprintListArgs, client: &JiraClient, output_mode: OutputMode) -> Result<()> {
    let sprints = client.list_sprints(args.board, args.state.as_deref()).await?;

    match output_mode {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&sprints)?);
        }
        OutputMode::Markdown => {
            println!("| ID | Name | State | Start | End |");
            println!("| --- | --- | --- | --- | --- |");
            for s in &sprints {
                let start = s.start_date.as_deref().unwrap_or("-");
                let end = s.end_date.as_deref().unwrap_or("-");
                println!("| {} | {} | {} | {} | {} |", s.id, s.name, s.state, start, end);
            }
        }
        OutputMode::Table | OutputMode::Plain => {
            println!("{}", render_sprints_table(&sprints));
        }
    }

    Ok(())
}

async fn handle_issues(args: SprintIssuesArgs, client: &JiraClient, output_mode: OutputMode) -> Result<()> {
    let results = client.list_sprint_issues(args.id).await?;

    match output_mode {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        OutputMode::Markdown | OutputMode::Table | OutputMode::Plain => {
            println!("{}", render_issues_table(&results.issues));
        }
    }

    Ok(())
}

async fn handle_boards(client: &JiraClient, output_mode: OutputMode) -> Result<()> {
    let boards = client.list_boards().await?;

    match output_mode {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&boards)?);
        }
        OutputMode::Markdown => {
            println!("| ID | Name | Type |");
            println!("| --- | --- | --- |");
            for b in &boards {
                println!("| {} | {} | {} |", b.id, b.name, b.board_type);
            }
        }
        OutputMode::Table | OutputMode::Plain => {
            println!("{}", render_boards_table(&boards));
        }
    }

    Ok(())
}
