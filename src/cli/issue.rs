//! Issue management subcommands (`jira issue`).
//!
//! Covers querying, viewing, creating, editing, assigning, transitioning,
//! commenting, time logging, and browser launching for Jira issues.

use crate::adf::from_text::text_to_adf;
use crate::client::JiraClient;
use crate::error::{JiraError, Result};
use crate::ui::format::OutputMode;
use crate::ui::{
    render_attachments_table, render_changelog_table, render_issue_detail,
    render_issue_links_table, render_issues_table, render_transitions_table, render_watchers_table,
};
use clap::{Args, Subcommand};
use colored::Colorize;

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

    /// Priority name (e.g. 'High', 'Medium', 'Low').
    #[arg(long)]
    pub priority: Option<String>,

    /// Assignee email, query, or '@me'.
    #[arg(short, long)]
    pub assignee: Option<String>,
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

    /// New priority.
    #[arg(long)]
    pub priority: Option<String>,
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
            handle_create(create_args, client, default_project).await
        }
        IssueSubcommand::Edit(edit_args) => handle_edit(edit_args, client).await,
        IssueSubcommand::Delete(delete_args) => handle_delete(delete_args, client).await,
        IssueSubcommand::Assign(assign_args) => handle_assign(assign_args, client).await,
        IssueSubcommand::Transition(trans_args) => handle_transition(trans_args, client).await,
        IssueSubcommand::Comment(comment_args) => handle_comment(comment_args, client).await,
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

async fn handle_create(
    args: IssueCreateArgs,
    client: &JiraClient,
    default_project: Option<&str>,
) -> Result<()> {
    let project = args
        .project
        .or_else(|| default_project.map(|p| p.to_string()))
        .ok_or_else(|| {
            JiraError::Config(
                "Project key must be specified via --project or default_project in config"
                    .to_string(),
            )
        })?;

    let issue_type = args.issue_type.unwrap_or_else(|| "Task".to_string());

    let summary = match args.summary {
        Some(s) => s,
        None => inquire::Text::new("Issue summary:")
            .prompt()
            .map_err(|e| JiraError::Prompt(e.to_string()))?,
    };

    let description_doc = args.description.as_deref().map(text_to_adf);

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

    let created = client
        .create_issue(
            &project,
            &issue_type,
            &summary,
            description_doc.as_ref(),
            args.priority.as_deref(),
            assignee_id.as_deref(),
        )
        .await?;

    let issue_url = format!("{}/browse/{}", client.instance_url(), created.key);
    println!(
        "{} Created issue {} ({})",
        "✔".green().bold(),
        created.key.bold().cyan(),
        issue_url.dimmed()
    );

    Ok(())
}

async fn handle_edit(args: IssueEditArgs, client: &JiraClient) -> Result<()> {
    let key = args.key.trim().to_uppercase();

    if args.summary.is_none() && args.description.is_none() && args.priority.is_none() {
        println!(
            "{} No update fields specified for {}. Provide at least one of -s/--summary, -d/--description, or --priority.",
            "⚠".yellow().bold(),
            key.cyan()
        );
        return Ok(());
    }

    let desc_doc = args.description.as_deref().map(text_to_adf);

    client
        .edit_issue(
            &key,
            args.summary.as_deref(),
            desc_doc.as_ref(),
            args.priority.as_deref(),
        )
        .await?;

    println!("{} Updated issue {}", "✔".green().bold(), key.cyan());
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

async fn handle_comment(args: IssueCommentArgs, client: &JiraClient) -> Result<()> {
    let key = args.key.trim().to_uppercase();
    match args.message {
        None => {
            let comments = client.get_comments(&key).await?;
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
                    println!("{} {}", author.bold().blue(), created.dimmed());
                    if let Some(body) = &comment.body {
                        println!("{}\n", crate::adf::adf_to_terminal(body, true));
                    }
                }
            }
        }
        Some(msg) => {
            let adf_body = text_to_adf(&msg);
            client.add_comment(&key, &adf_body).await?;
            println!("{} Added comment to {}", "✔".green().bold(), key.cyan());
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
    let comment_doc = args.comment.as_deref().map(text_to_adf);

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
    }
}
