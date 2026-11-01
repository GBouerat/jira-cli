//! Issue management subcommands (`jira issue`).
//!
//! Covers querying, viewing, creating, editing, assigning, transitioning,
//! commenting, time logging, and browser launching for Jira issues.

use crate::adf::from_text::text_to_adf;
use crate::cli::api::read_file_or_stdin;
use crate::client::JiraClient;
use crate::error::{JiraError, Result};
use crate::ui::format::OutputMode;
use crate::ui::{
    render_attachments_table, render_changelog_table, render_comments_table, render_issue_detail,
    render_issue_links_table, render_issues_table, render_transitions_table, render_watchers_table,
};
use clap::{Args, Subcommand};
use colored::Colorize;
use std::collections::HashSet;

/// Issue management commands.
#[derive(Debug, Clone, Args)]
pub struct IssueArgs {
    /// Issue operation to perform.
    #[command(subcommand)]
    pub command: IssueSubcommand,
}

/// Available issue subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum IssueSubcommand {
    /// Search and list issues matching JQL or filter flags.
    List(IssueListArgs),

    /// View detailed information, ADF description, and comments for an issue.
    View(IssueViewArgs),

    /// Create a new issue (interactively or via CLI flags).
    Create(IssueCreateArgs),

    /// Edit summary, description, or priority of an existing issue.
    Edit(IssueEditArgs),

    /// Delete an issue permanently.
    Delete(IssueDeleteArgs),

    /// Assign an issue to a user or '@me'.
    Assign(IssueAssignArgs),

    /// Transition an issue to another workflow status (or list transitions).
    Transition(IssueTransitionArgs),

    /// Add a comment to an issue or list comments.
    Comment(IssueCommentArgs),

    /// Fetch a single comment and optionally output its raw ADF body.
    #[command(name = "comment-get")]
    CommentGet(IssueCommentGetArgs),

    /// Edit an existing comment using Markdown or raw ADF JSON.
    #[command(name = "comment-edit")]
    CommentEdit(IssueCommentEditArgs),

    /// Delete a comment from an issue permanently.
    #[command(name = "comment-delete")]
    CommentDelete(IssueCommentDeleteArgs),

    /// Batch create subtasks under a parent issue from a JSON file.
    #[command(name = "create-subtasks")]
    CreateSubtasks(IssueCreateSubtasksArgs),

    /// Set priority across one or more issues.
    #[command(name = "set-priority")]
    SetPriority(IssueSetPriorityArgs),

    /// Log work time spent on an issue (e.g., '2h 30m').
    Worklog(IssueWorklogArgs),

    /// Open an issue in the default web browser.
    Open(IssueOpenArgs),

    /// Upload an attachment to an issue.
    Attach(IssueAttachArgs),

    /// List attachments belonging to an issue.
    Attachments(IssueAttachmentsArgs),

    /// Link two issues together with a named relationship.
    Link(IssueLinkArgs),

    /// List issue links for an issue.
    Links(IssueLinksArgs),

    /// Remove a link between two issues by link ID.
    Unlink(IssueUnlinkArgs),

    /// Add a watcher to an issue.
    Watch(IssueWatchArgs),

    /// Remove a watcher from an issue.
    Unwatch(IssueUnwatchArgs),

    /// List watchers for an issue.
    Watchers(IssueWatchersArgs),

    /// View field modification history (changelog) for an issue.
    History(IssueHistoryArgs),
}

/// Arguments for `jira issue list`.
#[derive(Debug, Clone, Args)]
pub struct IssueListArgs {
    /// Filter by project key (e.g., DEV).
    #[arg(short = 'P', long)]
    pub project: Option<String>,

    /// Filter by assignee account or '@me'.
    #[arg(short, long)]
    pub assignee: Option<String>,

    /// Filter by status name (e.g. 'In Progress', 'Done').
    #[arg(short, long)]
    pub status: Option<String>,

    /// Custom Jira Query Language (JQL) expression (overrides other filters).
    #[arg(short, long)]
    pub jql: Option<String>,

    /// Maximum number of issues to return.
    #[arg(short, long, default_value_t = 50)]
    pub limit: u32,
}

/// Arguments for `jira issue view <KEY>`.
#[derive(Debug, Clone, Args)]
pub struct IssueViewArgs {
    /// Issue key or ID (e.g., PROJ-123).
    pub key: String,
}

/// Arguments for `jira issue create`.
#[derive(Debug, Clone, Args)]
pub struct IssueCreateArgs {
    /// Project key (defaults to configured default project).
    #[arg(short = 'P', long)]
    pub project: Option<String>,

    /// Issue type (e.g. 'Task', 'Bug', 'Story').
    #[arg(short = 't', long)]
    pub issue_type: Option<String>,

    /// Issue summary / title.
    #[arg(short, long)]
    pub summary: Option<String>,

    /// Issue description in Markdown (compiled to ADF).
    #[arg(short, long)]
    pub description: Option<String>,

    /// Path to Markdown file containing description ('-' for stdin).
    #[arg(short = 'f', long = "body-file", conflicts_with = "description")]
    pub body_file: Option<String>,

    /// Priority name (e.g. 'High', 'Medium', 'Low').
    #[arg(long)]
    pub priority: Option<String>,

    /// Assignee email, query, or '@me'.
    #[arg(short, long)]
    pub assignee: Option<String>,

    /// Parent issue key to associate as a subtask.
    #[arg(long)]
    pub parent: Option<String>,

    /// Labels to assign to the issue (repeatable).
    #[arg(short = 'l', long = "label", action = clap::ArgAction::Append)]
    pub labels: Vec<String>,

    /// Custom or arbitrary fields to set in NAME=VALUE format (repeatable).
    #[arg(long = "field", action = clap::ArgAction::Append)]
    pub fields: Vec<String>,

    /// Output the payload without creating the issue.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Arguments for `jira issue edit <KEY>`.
#[derive(Debug, Clone, Args)]
pub struct IssueEditArgs {
    /// Issue key (e.g., PROJ-123).
    pub key: String,

    /// New summary / title.
    #[arg(short, long)]
    pub summary: Option<String>,

    /// New description in Markdown (compiled to ADF).
    #[arg(short, long)]
    pub description: Option<String>,

    /// Path to Markdown file containing updated description ('-' for stdin).
    #[arg(short = 'f', long = "body-file", conflicts_with = "description")]
    pub body_file: Option<String>,

    /// New priority.
    #[arg(long)]
    pub priority: Option<String>,

    /// Replace all issue labels with the provided list (repeatable).
    #[arg(short = 'l', long = "label", action = clap::ArgAction::Append, conflicts_with_all = ["add_labels", "remove_labels"])]
    pub labels: Vec<String>,

    /// Add specific labels using update verb (repeatable).
    #[arg(long = "add-label", action = clap::ArgAction::Append)]
    pub add_labels: Vec<String>,

    /// Remove specific labels using update verb (repeatable).
    #[arg(long = "remove-label", action = clap::ArgAction::Append)]
    pub remove_labels: Vec<String>,

    /// Custom or arbitrary fields to set in NAME=VALUE format (repeatable).
    #[arg(long = "field", action = clap::ArgAction::Append)]
    pub fields: Vec<String>,

    /// Output the payload without making update requests.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Arguments for `jira issue assign <KEY> [USER]`.
#[derive(Debug, Clone, Args)]
pub struct IssueAssignArgs {
    /// Issue key (e.g., PROJ-123).
    pub key: String,

    /// Assignee name, email, or '@me'. Defaults to '@me' if omitted and not unassigning.
    pub user: Option<String>,

    /// Unassign the issue.
    #[arg(long)]
    pub unassign: bool,
}

/// Arguments for `jira issue transition <KEY> [STATUS]`.
#[derive(Debug, Clone, Args)]
pub struct IssueTransitionArgs {
    /// Issue key (e.g., PROJ-123).
    pub key: String,

    /// Target transition name or ID. If omitted, lists available transitions.
    pub status: Option<String>,

    /// Optional comment to attach to the transition.
    #[arg(short = 'm', long)]
    pub comment: Option<String>,
}

/// Arguments for `jira issue comment <KEY> [MESSAGE]`.
#[derive(Debug, Clone, Args)]
pub struct IssueCommentArgs {
    /// Issue key (e.g., PROJ-123).
    pub key: String,

    /// Comment text in Markdown (compiled to ADF). If omitted, lists comments.
    pub message: Option<String>,

    /// Path to Markdown file containing comment text ('-' for stdin).
    #[arg(short = 'f', long = "body-file", conflicts_with = "message")]
    pub body_file: Option<String>,
}

/// Arguments for `jira issue comment-get <KEY> <COMMENT_ID>`.
#[derive(Debug, Clone, Args)]
pub struct IssueCommentGetArgs {
    /// Issue key or ID (e.g. PROJ-123).
    pub key: String,

    /// Unique comment ID to retrieve.
    pub comment_id: String,
}

/// Arguments for `jira issue comment-edit <KEY> <COMMENT_ID>`.
#[derive(Debug, Clone, Args)]
pub struct IssueCommentEditArgs {
    /// Issue key or ID (e.g. PROJ-123).
    pub key: String,

    /// Unique comment ID to edit.
    pub comment_id: String,

    /// Comment text in Markdown.
    #[arg(short = 'm', long = "message")]
    pub message: Option<String>,

    /// Path to file containing Markdown body ('-' for stdin).
    #[arg(short = 'f', long = "body-file", conflicts_with = "message")]
    pub body_file: Option<String>,

    /// Path to file containing raw ADF JSON sent untouched ('-' for stdin).
    #[arg(long = "adf", conflicts_with_all = ["message", "body_file"])]
    pub adf: Option<String>,

    /// Output the payload without making the request.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Arguments for `jira issue comment-delete <KEY> <COMMENT_ID>`.
#[derive(Debug, Clone, Args)]
pub struct IssueCommentDeleteArgs {
    /// Issue key or ID (e.g. PROJ-123).
    pub key: String,

    /// Unique comment ID to delete.
    pub comment_id: String,

    /// Skip confirmation prompt.
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Arguments for `jira issue create-subtasks <PARENT> <FILE>`.
#[derive(Debug, Clone, Args)]
pub struct IssueCreateSubtasksArgs {
    /// Parent issue key (e.g. PROJ-123).
    pub parent: String,

    /// Path to JSON file containing array of subtask objects ('-' for stdin).
    pub file: String,

    /// Maximum number of subtasks to create.
    #[arg(long)]
    pub limit: Option<usize>,

    /// Optional labels to attach to each created subtask (repeatable).
    #[arg(short = 'l', long = "label", action = clap::ArgAction::Append)]
    pub labels: Vec<String>,

    /// Custom or arbitrary fields to set in NAME=VALUE format (repeatable).
    #[arg(long = "field", action = clap::ArgAction::Append)]
    pub fields: Vec<String>,

    /// Output the payload without creating any subtasks.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Arguments for `jira issue set-priority <PRIORITY> <KEYS...>`.
#[derive(Debug, Clone, Args)]
pub struct IssueSetPriorityArgs {
    /// Target priority name (e.g. 'High', 'Medium', 'Low').
    pub priority: String,

    /// One or more issue keys to update (e.g. PROJ-101 PROJ-102).
    #[arg(required = true)]
    pub keys: Vec<String>,
}

/// Individual subtask representation within a batch create JSON file.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SubtaskEntry {
    /// Subtask summary / title.
    pub summary: String,

    /// Optional subtask description in Markdown.
    #[serde(default)]
    pub description: Option<String>,

    /// Optional subtask priority name.
    #[serde(default)]
    pub priority: Option<String>,
}

/// Arguments for `jira issue worklog <KEY> <TIME>`.
#[derive(Debug, Clone, Args)]
pub struct IssueWorklogArgs {
    /// Issue key (e.g., PROJ-123).
    pub key: String,

    /// Time spent duration string (e.g. '2h', '30m', '1d 4h').
    pub time: String,

    /// Optional comment for the worklog in Markdown.
    #[arg(short = 'm', long)]
    pub comment: Option<String>,
}

/// Arguments for `jira issue open <KEY>`.
#[derive(Debug, Clone, Args)]
pub struct IssueOpenArgs {
    /// Issue key (e.g., PROJ-123).
    pub key: String,
}

/// Arguments for `jira issue delete <KEY>`.
#[derive(Debug, Clone, Args)]
pub struct IssueDeleteArgs {
    /// Issue key or ID (e.g. PROJ-123).
    pub key: String,

    /// Whether to delete associated subtasks.
    #[arg(long, default_value_t = false)]
    pub delete_subtasks: bool,

    /// Skip confirmation prompt.
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Arguments for `jira issue attach <KEY> <FILE>`.
#[derive(Debug, Clone, Args)]
pub struct IssueAttachArgs {
    /// Issue key or ID (e.g. PROJ-123).
    pub key: String,

    /// Path to the file to attach.
    pub file: std::path::PathBuf,
}

/// Arguments for `jira issue attachments <KEY>`.
#[derive(Debug, Clone, Args)]
pub struct IssueAttachmentsArgs {
    /// Issue key or ID (e.g. PROJ-123).
    pub key: String,
}

/// Arguments for `jira issue link <INWARD> <OUTWARD>`.
#[derive(Debug, Clone, Args)]
pub struct IssueLinkArgs {
    /// Source issue key or ID (inward issue).
    pub inward: String,

    /// Target issue key or ID (outward issue).
    pub outward: String,

    /// Relationship link type (e.g. 'Blocks', 'Relates', 'Duplicates').
    #[arg(short = 't', long, default_value = "Relates")]
    pub link_type: String,

    /// Optional comment explaining the relationship.
    #[arg(short = 'm', long)]
    pub comment: Option<String>,

    /// Path to Markdown file containing link comment ('-' for stdin).
    #[arg(short = 'f', long = "body-file", conflicts_with = "comment")]
    pub body_file: Option<String>,
}

/// Arguments for `jira issue links <KEY>`.
#[derive(Debug, Clone, Args)]
pub struct IssueLinksArgs {
    /// Issue key or ID (e.g. PROJ-123).
    pub key: String,
}

/// Arguments for `jira issue unlink <LINK_ID>`.
#[derive(Debug, Clone, Args)]
pub struct IssueUnlinkArgs {
    /// Unique issue link ID.
    pub link_id: String,
}

/// Arguments for `jira issue watch <KEY>`.
#[derive(Debug, Clone, Args)]
pub struct IssueWatchArgs {
    /// Issue key or ID (e.g. PROJ-123).
    pub key: String,

    /// Account ID or username to add as watcher (defaults to current user).
    #[arg(short = 'u', long)]
    pub user: Option<String>,
}

/// Arguments for `jira issue unwatch <KEY>`.
#[derive(Debug, Clone, Args)]
pub struct IssueUnwatchArgs {
    /// Issue key or ID (e.g. PROJ-123).
    pub key: String,

    /// Account ID or username to remove from watchers (defaults to current user).
    #[arg(short = 'u', long)]
    pub user: Option<String>,
}

/// Arguments for `jira issue watchers <KEY>`.
#[derive(Debug, Clone, Args)]
pub struct IssueWatchersArgs {
    /// Issue key or ID (e.g. PROJ-123).
    pub key: String,
}

/// Arguments for `jira issue history <KEY>`.
#[derive(Debug, Clone, Args)]
pub struct IssueHistoryArgs {
    /// Issue key or ID (e.g. PROJ-123).
    pub key: String,

    /// Maximum number of changelog entries to retrieve.
    #[arg(short, long, default_value_t = 50)]
    pub limit: u32,
}

/// Dispatches issue subcommands.
pub async fn handle_issue_command(
    args: IssueArgs,
    client: &JiraClient,
    output_mode: OutputMode,
    default_project: Option<&str>,
) -> Result<()> {
    match args.command {
        IssueSubcommand::List(list_args) => {
            handle_list(list_args, client, output_mode, default_project).await
        }
        IssueSubcommand::View(view_args) => handle_view(view_args, client, output_mode).await,
        IssueSubcommand::Create(create_args) => {
            handle_create(create_args, client, output_mode, default_project).await
        }
        IssueSubcommand::Edit(edit_args) => handle_edit(edit_args, client, output_mode).await,
        IssueSubcommand::Delete(delete_args) => handle_delete(delete_args, client).await,
        IssueSubcommand::Assign(assign_args) => handle_assign(assign_args, client).await,
        IssueSubcommand::Transition(trans_args) => handle_transition(trans_args, client).await,
        IssueSubcommand::Comment(comment_args) => {
            handle_comment(comment_args, client, output_mode).await
        }
        IssueSubcommand::CommentGet(comment_get_args) => {
            handle_comment_get(comment_get_args, client, output_mode).await
        }
        IssueSubcommand::CommentEdit(comment_edit_args) => {
            handle_comment_edit(comment_edit_args, client, output_mode).await
        }
        IssueSubcommand::CommentDelete(comment_delete_args) => {
            handle_comment_delete(comment_delete_args, client, output_mode).await
        }
        IssueSubcommand::CreateSubtasks(subtasks_args) => {
            handle_create_subtasks(subtasks_args, client, output_mode).await
        }
        IssueSubcommand::SetPriority(priority_args) => {
            handle_set_priority(priority_args, client, output_mode).await
        }
        IssueSubcommand::Worklog(worklog_args) => handle_worklog(worklog_args, client).await,
        IssueSubcommand::Open(open_args) => handle_open(open_args, client),
        IssueSubcommand::Attach(attach_args) => {
            handle_attach(attach_args, client, output_mode).await
        }
        IssueSubcommand::Attachments(attachments_args) => {
            handle_attachments(attachments_args, client, output_mode).await
        }
        IssueSubcommand::Link(link_args) => handle_link(link_args, client).await,
        IssueSubcommand::Links(links_args) => handle_links(links_args, client, output_mode).await,
        IssueSubcommand::Unlink(unlink_args) => handle_unlink(unlink_args, client).await,
        IssueSubcommand::Watch(watch_args) => handle_watch(watch_args, client).await,
        IssueSubcommand::Unwatch(unwatch_args) => handle_unwatch(unwatch_args, client).await,
        IssueSubcommand::Watchers(watchers_args) => {
            handle_watchers(watchers_args, client, output_mode).await
        }
        IssueSubcommand::History(history_args) => {
            handle_history(history_args, client, output_mode).await
        }
    }
}

async fn handle_list(
    args: IssueListArgs,
    client: &JiraClient,
    output_mode: OutputMode,
    default_project: Option<&str>,
) -> Result<()> {
    let jql = if let Some(custom_jql) = args.jql {
        custom_jql
    } else {
        let mut clauses = Vec::new();

        let proj = args.project.as_deref().or(default_project);
        if let Some(p) = proj {
            clauses.push(format!("project = \"{}\"", p));
        }

        if let Some(assignee) = args.assignee {
            if assignee.eq_ignore_ascii_case("@me") {
                clauses.push("assignee = currentUser()".to_string());
            } else {
                clauses.push(format!("assignee = \"{}\"", assignee));
            }
        }

        if let Some(status) = args.status {
            clauses.push(format!("status = \"{}\"", status));
        }

        if clauses.is_empty() {
            "ORDER BY updated DESC".to_string()
        } else {
            format!("{} ORDER BY updated DESC", clauses.join(" AND "))
        }
    };

    let results = client.search_issues(&jql, 0, args.limit).await?;

    match output_mode {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        OutputMode::Markdown => {
            println!("| Key | Type | Summary | Status | Priority | Assignee |");
            println!("| --- | --- | --- | --- | --- | --- |");
            for issue in &results.issues {
                let key = &issue.key;
                let itype = issue
                    .fields
                    .issuetype
                    .as_ref()
                    .map(|t| t.name.as_str())
                    .unwrap_or("-");
                let summary = &issue.fields.summary;
                let status = issue
                    .fields
                    .status
                    .as_ref()
                    .map(|s| s.name.as_str())
                    .unwrap_or("-");
                let priority = issue
                    .fields
                    .priority
                    .as_ref()
                    .map(|p| p.name.as_str())
                    .unwrap_or("-");
                let assignee = issue
                    .fields
                    .assignee
                    .as_ref()
                    .map(|u| u.display_name.as_str())
                    .unwrap_or("Unassigned");
                println!(
                    "| {} | {} | {} | {} | {} | {} |",
                    key, itype, summary, status, priority, assignee
                );
            }
        }
        OutputMode::Table | OutputMode::Plain => {
            println!("{}", render_issues_table(&results.issues));
        }
    }

    Ok(())
}

async fn handle_view(
    args: IssueViewArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    let key = args.key.trim().to_uppercase();
    let issue = client.get_issue(&key).await?;

    match output_mode {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&issue)?);
        }
        OutputMode::Table | OutputMode::Plain | OutputMode::Markdown => {
            println!("{}", render_issue_detail(&issue, client.instance_url()));
        }
    }

    Ok(())
}

/// Parses a "NAME=VALUE" field definition, converting VALUE into JSON if valid JSON, otherwise string.
pub fn parse_custom_field(pair: &str) -> Result<(String, serde_json::Value)> {
    let parts: Vec<&str> = pair.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(JiraError::Generic(format!(
            "Invalid field format '{}'. Expected NAME=VALUE.",
            pair
        )));
    }
    let name = parts[0].trim().to_string();
    if name.is_empty() {
        return Err(JiraError::Generic(format!(
            "Field name cannot be empty in '{}'.",
            pair
        )));
    }
    let raw_val = parts[1].trim();
    let json_val = if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw_val) {
        parsed
    } else {
        serde_json::Value::String(raw_val.to_string())
    };
    Ok((name, json_val))
}

/// Resolves the subtask issue type name for a project from createmeta.
pub async fn resolve_subtask_type(client: &JiraClient, project_key: &str) -> Result<String> {
    let types = client.get_create_meta_issue_types(project_key).await?;
    let subtask_type = types.into_iter().find(|t| t.subtask).map(|t| t.name);
    subtask_type.ok_or_else(|| {
        JiraError::Generic(format!(
            "No subtask issue type found for project '{}'",
            project_key
        ))
    })
}

async fn handle_create(
    args: IssueCreateArgs,
    client: &JiraClient,
    output_mode: OutputMode,
    default_project: Option<&str>,
) -> Result<()> {
    let parent_key = args.parent.map(|k| k.trim().to_uppercase());

    let project = if let Some(p) = args.project {
        p
    } else if let Some(ref pk) = parent_key {
        let parent_issue = client.get_issue_with_fields(pk, "project").await?;
        parent_issue.fields.project.map(|p| p.key).ok_or_else(|| {
            JiraError::Generic(format!("Could not determine project for parent '{}'", pk))
        })?
    } else {
        default_project.map(|p| p.to_string()).ok_or_else(|| {
            JiraError::Config(
                "Project key must be specified via --project or default_project in config"
                    .to_string(),
            )
        })?
    };

    let issue_type = if let Some(it) = args.issue_type {
        it
    } else if parent_key.is_some() {
        resolve_subtask_type(client, &project).await?
    } else {
        "Task".to_string()
    };

    let summary = match args.summary {
        Some(s) => s,
        None => inquire::Text::new("Issue summary:")
            .prompt()
            .map_err(|e| JiraError::Prompt(e.to_string()))?,
    };

    let description_doc = if let Some(ref file_path) = args.body_file {
        let content = read_file_or_stdin(file_path)?;
        Some(text_to_adf(&content))
    } else {
        args.description.as_deref().map(text_to_adf)
    };

    let assignee_id = if let Some(user_query) = args.assignee {
        if user_query.eq_ignore_ascii_case("@me") {
            let me = client.myself().await?;
            Some(me.account_id)
        } else {
            let matched = client.find_users(&user_query).await?;
            matched.first().map(|u| u.account_id.clone())
        }
    } else {
        None
    };

    let mut fields = serde_json::Map::new();
    fields.insert("project".to_string(), serde_json::json!({ "key": project }));
    fields.insert(
        "issuetype".to_string(),
        serde_json::json!({ "name": issue_type }),
    );
    fields.insert("summary".to_string(), serde_json::json!(summary));

    if let Some(ref desc) = description_doc {
        fields.insert("description".to_string(), serde_json::to_value(desc)?);
    }
    if let Some(ref prio) = args.priority {
        fields.insert("priority".to_string(), serde_json::json!({ "name": prio }));
    }
    if let Some(ref acc_id) = assignee_id {
        fields.insert(
            "assignee".to_string(),
            serde_json::json!({ "accountId": acc_id }),
        );
    }
    if let Some(ref pk) = parent_key {
        fields.insert("parent".to_string(), serde_json::json!({ "key": pk }));
    }
    if !args.labels.is_empty() {
        fields.insert("labels".to_string(), serde_json::json!(args.labels));
    }
    for custom in &args.fields {
        let (field_name, field_val) = parse_custom_field(custom)?;
        fields.insert(field_name, field_val);
    }

    let payload = serde_json::json!({ "fields": fields });

    if args.dry_run {
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    let created = client.create_issue_raw(&payload).await?;

    let issue_url = format!("{}/browse/{}", client.instance_url(), created.key);
    match output_mode {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&created)?);
        }
        _ => {
            println!(
                "{} Created issue {} ({})",
                "✔".green().bold(),
                created.key.bold().cyan(),
                issue_url.dimmed()
            );
        }
    }

    Ok(())
}

async fn handle_edit(
    args: IssueEditArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    let key = args.key.trim().to_uppercase();

    let description_doc = if let Some(ref file_path) = args.body_file {
        let content = read_file_or_stdin(file_path)?;
        Some(text_to_adf(&content))
    } else {
        args.description.as_deref().map(text_to_adf)
    };

    let mut fields = serde_json::Map::new();
    let mut update = serde_json::Map::new();

    if let Some(ref s) = args.summary {
        fields.insert("summary".to_string(), serde_json::json!(s));
    }
    if let Some(ref desc) = description_doc {
        fields.insert("description".to_string(), serde_json::to_value(desc)?);
    }
    if let Some(ref prio) = args.priority {
        fields.insert("priority".to_string(), serde_json::json!({ "name": prio }));
    }
    if !args.labels.is_empty() {
        fields.insert("labels".to_string(), serde_json::json!(args.labels));
    }
    for custom in &args.fields {
        let (field_name, field_val) = parse_custom_field(custom)?;
        fields.insert(field_name, field_val);
    }

    if !args.add_labels.is_empty() || !args.remove_labels.is_empty() {
        let mut ops = Vec::new();
        for l in &args.add_labels {
            ops.push(serde_json::json!({ "add": l }));
        }
        for l in &args.remove_labels {
            ops.push(serde_json::json!({ "remove": l }));
        }
        update.insert("labels".to_string(), serde_json::json!(ops));
    }

    if fields.is_empty() && update.is_empty() {
        if output_mode == OutputMode::Json {
            return Err(JiraError::Generic(format!(
                "No update fields specified for {}. Provide at least one of -s/--summary, -d/--description, -f/--body-file, --priority, --label, --add-label, --remove-label, or --field.",
                key
            )));
        } else {
            eprintln!(
                "{} No update fields specified for {}. Provide at least one of -s/--summary, -d/--description, -f/--body-file, --priority, --label, --add-label, --remove-label, or --field.",
                "⚠".yellow().bold(),
                key.cyan()
            );
            return Ok(());
        }
    }

    let mut payload_map = serde_json::Map::new();
    if !fields.is_empty() {
        payload_map.insert("fields".to_string(), serde_json::Value::Object(fields));
    }
    if !update.is_empty() {
        payload_map.insert("update".to_string(), serde_json::Value::Object(update));
    }
    let payload = serde_json::Value::Object(payload_map);

    if args.dry_run {
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    client.edit_issue_raw(&key, &payload).await?;

    match output_mode {
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::json!({
                    "updated": true,
                    "key": key
                })
            );
        }
        _ => {
            println!("{} Updated issue {}", "✔".green().bold(), key.cyan());
        }
    }

    Ok(())
}

async fn handle_assign(args: IssueAssignArgs, client: &JiraClient) -> Result<()> {
    let key = args.key.trim().to_uppercase();
    if args.unassign {
        client.assign_issue(&key, None).await?;
        println!("{} Unassigned {}", "✔".green().bold(), key.cyan());
        return Ok(());
    }

    let target_user = args.user.unwrap_or_else(|| "@me".to_string());
    let account_id = if target_user.eq_ignore_ascii_case("@me") {
        let me = client.myself().await?;
        me.account_id
    } else {
        let users = client.find_users(&target_user).await?;
        let user = users
            .first()
            .ok_or_else(|| JiraError::Generic(format!("User '{}' not found", target_user)))?;
        user.account_id.clone()
    };

    client.assign_issue(&key, Some(&account_id)).await?;
    println!(
        "{} Assigned {} to {}",
        "✔".green().bold(),
        key.cyan(),
        target_user.bold()
    );

    Ok(())
}

async fn handle_transition(args: IssueTransitionArgs, client: &JiraClient) -> Result<()> {
    let key = args.key.trim().to_uppercase();
    let transitions = client.get_transitions(&key).await?;

    match args.status {
        None => {
            println!("Available transitions for {}:\n", key.cyan());
            println!("{}", render_transitions_table(&transitions));
            println!(
                "\nAdvance status with: {}",
                format!("jira issue transition {} <NAME_OR_ID>", key).yellow()
            );
        }
        Some(target) => {
            let matched = transitions
                .iter()
                .find(|t| {
                    t.id == target
                        || t.name.eq_ignore_ascii_case(&target)
                        || t.to.name.eq_ignore_ascii_case(&target)
                })
                .ok_or_else(|| {
                    JiraError::Generic(format!(
                        "Transition '{}' not available for {}. Run 'jira issue transition {}' to see options.",
                        target, key, key
                    ))
                })?;

            let comment_doc = args.comment.as_deref().map(text_to_adf);
            client
                .do_transition(&key, &matched.id, comment_doc.as_ref())
                .await?;

            println!(
                "{} Transitioned {} to status '{}'",
                "✔".green().bold(),
                key.cyan(),
                matched.to.name.green().bold()
            );
        }
    }

    Ok(())
}

async fn handle_comment(
    args: IssueCommentArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    let key = args.key.trim().to_uppercase();

    let comment_body = if let Some(ref file_path) = args.body_file {
        let content = read_file_or_stdin(file_path)?;
        Some(content)
    } else {
        args.message
    };

    match comment_body {
        None => {
            let comments = client.get_comments(&key).await?;
            match output_mode {
                OutputMode::Json => {
                    println!("{}", serde_json::to_string_pretty(&comments)?);
                }
                OutputMode::Table => {
                    println!("{}", render_comments_table(&comments.comments));
                }
                OutputMode::Markdown => {
                    println!("| ID | Author | Created |");
                    println!("| --- | --- | --- |");
                    for c in &comments.comments {
                        let author = c
                            .author
                            .as_ref()
                            .map(|u| u.display_name.as_str())
                            .unwrap_or("Anonymous");
                        let created = c.created.as_deref().unwrap_or("-");
                        println!("| {} | {} | {} |", c.id, author, created);
                    }
                }
                OutputMode::Plain => {
                    if comments.comments.is_empty() {
                        println!("No comments on {}.", key.cyan());
                    } else {
                        println!("Comments for {}:\n", key.cyan());
                        for comment in &comments.comments {
                            let author = comment
                                .author
                                .as_ref()
                                .map(|u| u.display_name.as_str())
                                .unwrap_or("Anonymous");
                            let created = comment.created.as_deref().unwrap_or("");
                            println!(
                                "[ID: {}] {} {}",
                                comment.id.yellow().bold(),
                                author.bold().blue(),
                                created.dimmed()
                            );
                            if let Some(body) = &comment.body {
                                println!("{}\n", crate::adf::adf_to_terminal(body, true));
                            }
                        }
                    }
                }
            }
        }
        Some(msg) => {
            if msg.trim().is_empty() {
                return Err(JiraError::Generic(
                    "Comment message cannot be empty.".to_string(),
                ));
            }
            let adf_body = text_to_adf(&msg);
            let created_comment = client.add_comment(&key, &adf_body).await?;
            match output_mode {
                OutputMode::Json => {
                    println!("{}", serde_json::to_string_pretty(&created_comment)?);
                }
                _ => {
                    println!(
                        "{} Added comment (ID: {}) to {}",
                        "✔".green().bold(),
                        created_comment.id.cyan(),
                        key.cyan()
                    );
                }
            }
        }
    }

    Ok(())
}

async fn handle_comment_get(
    args: IssueCommentGetArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    let key = args.key.trim().to_uppercase();
    let comment = client.get_comment(&key, &args.comment_id).await?;

    match output_mode {
        OutputMode::Json => {
            if let Some(ref body) = comment.body {
                println!("{}", serde_json::to_string_pretty(body)?);
            } else {
                println!("null");
            }
        }
        OutputMode::Table | OutputMode::Plain | OutputMode::Markdown => {
            let author = comment
                .author
                .as_ref()
                .map(|u| u.display_name.as_str())
                .unwrap_or("Anonymous");
            let created = comment.created.as_deref().unwrap_or("");
            println!(
                "[ID: {}] {} {}",
                comment.id.yellow().bold(),
                author.bold().blue(),
                created.dimmed()
            );
            if let Some(ref body) = comment.body {
                println!("\n{}", crate::adf::adf_to_terminal(body, true));
            }
        }
    }

    Ok(())
}

async fn handle_comment_edit(
    args: IssueCommentEditArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    let key = args.key.trim().to_uppercase();
    let comment_id = args.comment_id.trim();

    let adf_val = if let Some(ref adf_path) = args.adf {
        let raw = read_file_or_stdin(adf_path)?;
        let parsed: serde_json::Value = serde_json::from_str(&raw).map_err(|e| {
            JiraError::Generic(format!("Invalid raw ADF JSON in '{}': {}", adf_path, e))
        })?;
        parsed
    } else if let Some(ref body_path) = args.body_file {
        let raw = read_file_or_stdin(body_path)?;
        if raw.trim().is_empty() {
            return Err(JiraError::Generic(
                "Comment body cannot be empty.".to_string(),
            ));
        }
        let doc = text_to_adf(&raw);
        serde_json::to_value(&doc)?
    } else if let Some(ref msg) = args.message {
        if msg.trim().is_empty() {
            return Err(JiraError::Generic(
                "Comment message cannot be empty.".to_string(),
            ));
        }
        let doc = text_to_adf(msg);
        serde_json::to_value(&doc)?
    } else {
        return Err(JiraError::Generic(
            "Comment body must be provided via -f/--body-file, --adf, or -m/--message".to_string(),
        ));
    };

    let payload = serde_json::json!({
        "body": adf_val
    });

    if args.dry_run {
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    let updated = client.edit_comment_raw(&key, comment_id, &payload).await?;

    match output_mode {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&updated)?);
        }
        _ => {
            println!(
                "{} Updated comment {} on {}",
                "✔".green().bold(),
                comment_id.cyan(),
                key.cyan()
            );
        }
    }

    Ok(())
}

async fn handle_comment_delete(
    args: IssueCommentDeleteArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    let key = args.key.trim().to_uppercase();
    let comment_id = args.comment_id.trim();

    if !args.yes {
        let confirmed = inquire::Confirm::new(&format!(
            "Permanently delete comment {} on {}?",
            comment_id, key
        ))
        .with_default(false)
        .prompt()
        .map_err(|e| JiraError::Prompt(e.to_string()))?;

        if !confirmed {
            println!("Comment deletion cancelled.");
            return Ok(());
        }
    }

    client.delete_comment(&key, comment_id).await?;

    match output_mode {
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::json!({
                    "deleted": true,
                    "key": key,
                    "comment_id": comment_id
                })
            );
        }
        _ => {
            println!(
                "{} Deleted comment {} on {}",
                "✔".green().bold(),
                comment_id.cyan(),
                key.cyan()
            );
        }
    }

    Ok(())
}

async fn handle_create_subtasks(
    args: IssueCreateSubtasksArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    let parent_key = args.parent.trim().to_uppercase();

    // 1. Fetch parent issue to retrieve project and existing subtasks
    let parent_issue = client
        .get_issue_with_fields(&parent_key, "project,subtasks")
        .await?;

    let project_key = parent_issue
        .fields
        .project
        .as_ref()
        .map(|p| p.key.clone())
        .ok_or_else(|| {
            JiraError::Generic(format!(
                "Could not determine project for parent issue '{}'",
                parent_key
            ))
        })?;

    let mut existing_summaries = HashSet::new();
    if let Some(subtasks) = parent_issue.fields.subtasks {
        for s in subtasks {
            if let Some(f) = s.fields {
                existing_summaries.insert(f.summary.trim().to_string());
            }
        }
    }

    // 2. Resolve subtask issue type
    let subtask_type = resolve_subtask_type(client, &project_key).await?;

    // 3. Read and parse entries JSON file
    let file_content = read_file_or_stdin(&args.file)?;
    let entries: Vec<SubtaskEntry> = serde_json::from_str(&file_content).map_err(|e| {
        JiraError::Generic(format!(
            "Failed to parse subtasks JSON from '{}': {}",
            args.file, e
        ))
    })?;

    // 4. Pre-parse custom fields
    let mut custom_fields = serde_json::Map::new();
    for f in &args.fields {
        let (name, val) = parse_custom_field(f)?;
        custom_fields.insert(name, val);
    }

    // 5. Create subtasks or report dry-run
    let mut created_results: Vec<serde_json::Value> = Vec::new();
    let mut skipped_summaries: Vec<String> = Vec::new();

    for entry in entries {
        let trimmed_summary = entry.summary.trim();
        if existing_summaries.contains(trimmed_summary) {
            if output_mode != OutputMode::Json {
                println!(
                    "{} Skipped existing subtask: '{}'",
                    "↷".yellow().bold(),
                    entry.summary
                );
            }
            skipped_summaries.push(entry.summary);
            continue;
        }

        if let Some(limit) = args.limit {
            if created_results.len() >= limit {
                break;
            }
        }

        let mut fields = custom_fields.clone();
        fields.insert(
            "project".to_string(),
            serde_json::json!({ "key": project_key }),
        );
        fields.insert(
            "parent".to_string(),
            serde_json::json!({ "key": parent_key }),
        );
        fields.insert(
            "issuetype".to_string(),
            serde_json::json!({ "name": subtask_type }),
        );
        fields.insert("summary".to_string(), serde_json::json!(entry.summary));

        if let Some(ref desc) = entry.description {
            let doc = text_to_adf(desc);
            fields.insert("description".to_string(), serde_json::to_value(doc)?);
        }

        if let Some(ref prio) = entry.priority {
            fields.insert("priority".to_string(), serde_json::json!({ "name": prio }));
        }

        if !args.labels.is_empty() {
            fields.insert("labels".to_string(), serde_json::json!(args.labels));
        }

        let payload = serde_json::json!({ "fields": fields });

        if args.dry_run {
            let dry_key = format!("{}-DRY", parent_key);
            let dry_url = format!("{}/browse/{}", client.instance_url(), dry_key);
            if output_mode != OutputMode::Json {
                println!(
                    "{} [dry-run] Would create subtask '{}' under {}",
                    "ℹ".blue().bold(),
                    entry.summary,
                    parent_key.cyan()
                );
                println!("{}", serde_json::to_string_pretty(&payload)?);
            }
            created_results.push(serde_json::json!({
                "key": dry_key,
                "url": dry_url,
                "summary": entry.summary
            }));
        } else {
            let created = client.create_issue_raw(&payload).await?;
            let issue_url = format!("{}/browse/{}", client.instance_url(), created.key);
            if output_mode != OutputMode::Json {
                println!(
                    "{} Created {} ({}) - {}",
                    "✔".green().bold(),
                    created.key.bold().cyan(),
                    issue_url.dimmed(),
                    entry.summary
                );
            }
            created_results.push(serde_json::json!({
                "key": created.key,
                "url": issue_url,
                "summary": entry.summary
            }));
            existing_summaries.insert(trimmed_summary.to_string());
        }
    }

    if output_mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "created": created_results,
                "skipped": skipped_summaries
            })
        );
    }

    Ok(())
}

async fn handle_set_priority(
    args: IssueSetPriorityArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    let mut updated_keys = Vec::new();

    for key_raw in &args.keys {
        let key = key_raw.trim().to_uppercase();
        client
            .edit_issue(&key, None, None, Some(&args.priority))
            .await?;
        updated_keys.push(key);
    }

    match output_mode {
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::json!({
                    "priority": args.priority,
                    "updated": updated_keys
                })
            );
        }
        _ => {
            for key in &updated_keys {
                println!(
                    "{} Set priority of {} to '{}'",
                    "✔".green().bold(),
                    key.cyan(),
                    args.priority.bold()
                );
            }
        }
    }

    Ok(())
}

async fn handle_worklog(args: IssueWorklogArgs, client: &JiraClient) -> Result<()> {
    let key = args.key.trim().to_uppercase();
    let comment_doc = args.comment.as_deref().map(text_to_adf);
    let worklog = client
        .add_worklog(&key, &args.time, comment_doc.as_ref())
        .await?;

    println!(
        "{} Logged {} on issue {}",
        "✔".green().bold(),
        worklog.time_spent.bold().green(),
        key.cyan()
    );

    Ok(())
}

fn handle_open(args: IssueOpenArgs, client: &JiraClient) -> Result<()> {
    let key = args.key.trim().to_uppercase();
    let url = format!("{}/browse/{}", client.instance_url(), key);
    println!("Opening {} in web browser...", url.cyan());
    open::that(&url).map_err(|e| JiraError::Generic(format!("Failed to open browser: {}", e)))?;
    Ok(())
}

async fn handle_delete(args: IssueDeleteArgs, client: &JiraClient) -> Result<()> {
    let key = args.key.trim().to_uppercase();
    if !args.yes {
        let confirmed = inquire::Confirm::new(&format!("Permanently delete issue {}?", key))
            .with_default(false)
            .prompt()
            .map_err(|e| JiraError::Prompt(e.to_string()))?;
        if !confirmed {
            println!("Deletion cancelled.");
            return Ok(());
        }
    }
    client.delete_issue(&key, args.delete_subtasks).await?;
    println!("{} Deleted issue {}.", "✔".green().bold(), key.cyan());
    Ok(())
}

async fn handle_attach(
    args: IssueAttachArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    let key = args.key.trim().to_uppercase();
    if !args.file.exists() {
        return Err(JiraError::Generic(format!(
            "File not found: {}",
            args.file.display()
        )));
    }
    let filename = args
        .file
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| JiraError::Generic("Invalid filename".to_string()))?;
    let bytes = std::fs::read(&args.file).map_err(|e| {
        JiraError::Generic(format!(
            "Failed to read file {}: {}",
            args.file.display(),
            e
        ))
    })?;

    let attachments = client.add_attachment(&key, filename, bytes).await?;

    match output_mode {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&attachments)?);
        }
        OutputMode::Table | OutputMode::Plain | OutputMode::Markdown => {
            println!(
                "{} Attached {} to issue {}.",
                "✔".green().bold(),
                filename.bold(),
                key.cyan()
            );
        }
    }
    Ok(())
}

async fn handle_attachments(
    args: IssueAttachmentsArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    let key = args.key.trim().to_uppercase();
    let attachments = client.get_attachments(&key).await?;

    match output_mode {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&attachments)?);
        }
        OutputMode::Markdown => {
            println!("| ID | Filename | Size | MIME Type | Author | Created |");
            println!("| --- | --- | --- | --- | --- | --- |");
            for a in &attachments {
                let author = a
                    .author
                    .as_ref()
                    .map(|u| u.display_name.as_str())
                    .unwrap_or("-");
                let mime = a.mime_type.as_deref().unwrap_or("-");
                let created = a.created.as_deref().unwrap_or("-");
                println!(
                    "| {} | {} | {} | {} | {} | {} |",
                    a.id, a.filename, a.size, mime, author, created
                );
            }
        }
        OutputMode::Table | OutputMode::Plain => {
            println!("{}", render_attachments_table(&attachments));
        }
    }
    Ok(())
}

async fn handle_link(args: IssueLinkArgs, client: &JiraClient) -> Result<()> {
    let inward = args.inward.trim().to_uppercase();
    let outward = args.outward.trim().to_uppercase();
    let comment_doc = if let Some(ref path) = args.body_file {
        let raw = read_file_or_stdin(path)?;
        Some(text_to_adf(&raw))
    } else {
        args.comment.as_deref().map(text_to_adf)
    };

    client
        .link_issues(&inward, &outward, &args.link_type, comment_doc.as_ref())
        .await?;

    println!(
        "{} Linked {} and {} with relation '{}'.",
        "✔".green().bold(),
        inward.cyan(),
        outward.cyan(),
        args.link_type.bold()
    );
    Ok(())
}

async fn handle_links(
    args: IssueLinksArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    let key = args.key.trim().to_uppercase();
    let links = client.get_issue_links(&key).await?;

    match output_mode {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&links)?);
        }
        OutputMode::Markdown => {
            println!("| Link ID | Relationship | Issue | Summary | Status |");
            println!("| --- | --- | --- | --- | --- |");
            for l in &links {
                let (rel, target) = if let Some(ref o) = l.outward_issue {
                    (&l.link_type.outward, Some(o))
                } else if let Some(ref i) = l.inward_issue {
                    (&l.link_type.inward, Some(i))
                } else {
                    (&l.link_type.name, None)
                };
                let target_key = target.map(|t| t.key.as_str()).unwrap_or("-");
                let summary = target
                    .and_then(|t| t.fields.as_ref())
                    .map(|f| f.summary.as_str())
                    .unwrap_or("-");
                let status = target
                    .and_then(|t| t.fields.as_ref())
                    .and_then(|f| f.status.as_ref())
                    .map(|s| s.name.as_str())
                    .unwrap_or("-");
                println!(
                    "| {} | {} | {} | {} | {} |",
                    l.id, rel, target_key, summary, status
                );
            }
        }
        OutputMode::Table | OutputMode::Plain => {
            println!("{}", render_issue_links_table(&links, &key));
        }
    }
    Ok(())
}

async fn handle_unlink(args: IssueUnlinkArgs, client: &JiraClient) -> Result<()> {
    client.delete_issue_link(&args.link_id).await?;
    println!(
        "{} Deleted issue link {}.",
        "✔".green().bold(),
        args.link_id.cyan()
    );
    Ok(())
}

async fn handle_watch(args: IssueWatchArgs, client: &JiraClient) -> Result<()> {
    let key = args.key.trim().to_uppercase();
    let account_id = match args.user {
        Some(ref u) if u.eq_ignore_ascii_case("@me") => client.myself().await?.account_id,
        Some(ref u) => {
            let found = client.find_users(u).await?;
            found
                .into_iter()
                .next()
                .map(|usr| usr.account_id)
                .unwrap_or_else(|| u.clone())
        }
        None => client.myself().await?.account_id,
    };

    client.add_watcher(&key, &account_id).await?;
    println!(
        "{} Added watcher to issue {}.",
        "✔".green().bold(),
        key.cyan()
    );
    Ok(())
}

async fn handle_unwatch(args: IssueUnwatchArgs, client: &JiraClient) -> Result<()> {
    let key = args.key.trim().to_uppercase();
    let account_id = match args.user {
        Some(ref u) if u.eq_ignore_ascii_case("@me") => client.myself().await?.account_id,
        Some(ref u) => {
            let found = client.find_users(u).await?;
            found
                .into_iter()
                .next()
                .map(|usr| usr.account_id)
                .unwrap_or_else(|| u.clone())
        }
        None => client.myself().await?.account_id,
    };

    client.remove_watcher(&key, &account_id).await?;
    println!(
        "{} Removed watcher from issue {}.",
        "✔".green().bold(),
        key.cyan()
    );
    Ok(())
}

async fn handle_watchers(
    args: IssueWatchersArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    let key = args.key.trim().to_uppercase();
    let watchers = client.get_watchers(&key).await?;

    match output_mode {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&watchers)?);
        }
        OutputMode::Markdown => {
            println!("| Account ID | Display Name | Active |");
            println!("| --- | --- | --- |");
            for w in &watchers.watchers {
                let act = if w.active { "yes" } else { "no" };
                println!("| {} | {} | {} |", w.account_id, w.display_name, act);
            }
        }
        OutputMode::Table | OutputMode::Plain => {
            println!("{}", render_watchers_table(&watchers));
        }
    }
    Ok(())
}

async fn handle_history(
    args: IssueHistoryArgs,
    client: &JiraClient,
    output_mode: OutputMode,
) -> Result<()> {
    let key = args.key.trim().to_uppercase();
    let changelogs = client.get_issue_changelog(&key, args.limit).await?;

    match output_mode {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&changelogs)?);
        }
        OutputMode::Markdown => {
            println!("| Date | Author | Field | From | To |");
            println!("| --- | --- | --- | --- | --- |");
            for c in &changelogs {
                let author = c
                    .author
                    .as_ref()
                    .map(|u| u.display_name.as_str())
                    .unwrap_or("System");
                for item in &c.items {
                    let from = item.from_string.as_deref().unwrap_or("-");
                    let to = item.to_string.as_deref().unwrap_or("-");
                    println!(
                        "| {} | {} | {} | {} | {} |",
                        c.created, author, item.field, from, to
                    );
                }
            }
        }
        OutputMode::Table | OutputMode::Plain => {
            println!("{}", render_changelog_table(&changelogs));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct TestCli {
        #[command(subcommand)]
        command: IssueSubcommand,
    }

    #[test]
    fn test_issue_subcommand_parsing() {
        let cli = TestCli::try_parse_from(["jira", "delete", "DEV-123", "-y"]).unwrap();
        match cli.command {
            IssueSubcommand::Delete(args) => {
                assert_eq!(args.key, "DEV-123");
                assert!(args.yes);
            }
            _ => panic!("unexpected command parsed"),
        }

        let cli =
            TestCli::try_parse_from(["jira", "link", "DEV-1", "DEV-2", "-t", "Blocks"]).unwrap();
        match cli.command {
            IssueSubcommand::Link(args) => {
                assert_eq!(args.inward, "DEV-1");
                assert_eq!(args.outward, "DEV-2");
                assert_eq!(args.link_type, "Blocks");
            }
            _ => panic!("unexpected command parsed"),
        }

        let cli = TestCli::try_parse_from(["jira", "watch", "DEV-1", "-u", "557058:xxx"]).unwrap();
        match cli.command {
            IssueSubcommand::Watch(args) => {
                assert_eq!(args.key, "DEV-1");
                assert_eq!(args.user.as_deref(), Some("557058:xxx"));
            }
            _ => panic!("unexpected command parsed"),
        }

        let cli = TestCli::try_parse_from(["jira", "history", "DEV-1", "-l", "20"]).unwrap();
        match cli.command {
            IssueSubcommand::History(args) => {
                assert_eq!(args.key, "DEV-1");
                assert_eq!(args.limit, 20);
            }
            _ => panic!("unexpected command parsed"),
        }

        let cli = TestCli::try_parse_from(["jira", "comment-get", "DEV-1", "10050"]).unwrap();
        match cli.command {
            IssueSubcommand::CommentGet(args) => {
                assert_eq!(args.key, "DEV-1");
                assert_eq!(args.comment_id, "10050");
            }
            _ => panic!("unexpected command parsed"),
        }

        let cli = TestCli::try_parse_from([
            "jira",
            "comment-edit",
            "DEV-1",
            "10050",
            "-f",
            "updated.md",
            "--dry-run",
        ])
        .unwrap();
        match cli.command {
            IssueSubcommand::CommentEdit(args) => {
                assert_eq!(args.key, "DEV-1");
                assert_eq!(args.comment_id, "10050");
                assert_eq!(args.body_file.as_deref(), Some("updated.md"));
                assert!(args.dry_run);
            }
            _ => panic!("unexpected command parsed"),
        }

        let cli = TestCli::try_parse_from([
            "jira",
            "comment-edit",
            "DEV-1",
            "10050",
            "--adf",
            "raw_adf.json",
        ])
        .unwrap();
        match cli.command {
            IssueSubcommand::CommentEdit(args) => {
                assert_eq!(args.adf.as_deref(), Some("raw_adf.json"));
            }
            _ => panic!("unexpected command parsed"),
        }

        let cli =
            TestCli::try_parse_from(["jira", "comment-delete", "DEV-1", "10050", "-y"]).unwrap();
        match cli.command {
            IssueSubcommand::CommentDelete(args) => {
                assert_eq!(args.key, "DEV-1");
                assert_eq!(args.comment_id, "10050");
                assert!(args.yes);
            }
            _ => panic!("unexpected command parsed"),
        }

        let cli = TestCli::try_parse_from([
            "jira",
            "create",
            "-P",
            "DEV",
            "-s",
            "Subtask item",
            "--parent",
            "DEV-100",
            "-l",
            "core",
            "-l",
            "backend",
            "--field",
            "customfield_123=simple_string",
            "--field",
            "customfield_456={\"key\":\"val\"}",
            "--dry-run",
        ])
        .unwrap();
        match cli.command {
            IssueSubcommand::Create(args) => {
                assert_eq!(args.project.as_deref(), Some("DEV"));
                assert_eq!(args.summary.as_deref(), Some("Subtask item"));
                assert_eq!(args.parent.as_deref(), Some("DEV-100"));
                assert_eq!(args.labels, vec!["core", "backend"]);
                assert_eq!(
                    args.fields,
                    vec![
                        "customfield_123=simple_string",
                        "customfield_456={\"key\":\"val\"}"
                    ]
                );
                assert!(args.dry_run);
            }
            _ => panic!("unexpected command parsed"),
        }

        let cli = TestCli::try_parse_from([
            "jira",
            "edit",
            "DEV-101",
            "-f",
            "body.md",
            "--add-label",
            "new-tag",
            "--remove-label",
            "old-tag",
            "--field",
            "priority=High",
            "--dry-run",
        ])
        .unwrap();
        match cli.command {
            IssueSubcommand::Edit(args) => {
                assert_eq!(args.key, "DEV-101");
                assert_eq!(args.body_file.as_deref(), Some("body.md"));
                assert_eq!(args.add_labels, vec!["new-tag"]);
                assert_eq!(args.remove_labels, vec!["old-tag"]);
                assert_eq!(args.fields, vec!["priority=High"]);
                assert!(args.dry_run);
            }
            _ => panic!("unexpected command parsed"),
        }

        let cli = TestCli::try_parse_from([
            "jira",
            "create-subtasks",
            "DEV-100",
            "tasks.json",
            "--limit",
            "5",
            "-l",
            "batch",
            "--field",
            "custom=123",
            "--dry-run",
        ])
        .unwrap();
        match cli.command {
            IssueSubcommand::CreateSubtasks(args) => {
                assert_eq!(args.parent, "DEV-100");
                assert_eq!(args.file, "tasks.json");
                assert_eq!(args.limit, Some(5));
                assert_eq!(args.labels, vec!["batch"]);
                assert_eq!(args.fields, vec!["custom=123"]);
                assert!(args.dry_run);
            }
            _ => panic!("unexpected command parsed"),
        }

        let cli = TestCli::try_parse_from([
            "jira",
            "set-priority",
            "High",
            "DEV-101",
            "DEV-102",
            "DEV-103",
        ])
        .unwrap();
        match cli.command {
            IssueSubcommand::SetPriority(args) => {
                assert_eq!(args.priority, "High");
                assert_eq!(args.keys, vec!["DEV-101", "DEV-102", "DEV-103"]);
            }
            _ => panic!("unexpected command parsed"),
        }
    }

    #[test]
    fn test_parse_custom_field() {
        let (name, val) = parse_custom_field("customfield_16375=N/A").unwrap();
        assert_eq!(name, "customfield_16375");
        assert_eq!(val, serde_json::Value::String("N/A".to_string()));

        let (name, val) =
            parse_custom_field("components=[{\"name\":\"Backend\"},{\"name\":\"API\"}]").unwrap();
        assert_eq!(name, "components");
        assert!(val.is_array());
        assert_eq!(val[0]["name"], "Backend");

        let (name, val) = parse_custom_field("customfield_number=42").unwrap();
        assert_eq!(name, "customfield_number");
        assert_eq!(val, serde_json::json!(42));

        let (name, val) = parse_custom_field("customfield_bool=true").unwrap();
        assert_eq!(name, "customfield_bool");
        assert_eq!(val, serde_json::json!(true));

        assert!(parse_custom_field("invalid_no_equals").is_err());
        assert!(parse_custom_field("=val_no_name").is_err());
    }
}
