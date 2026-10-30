//! Multi-section detail inspector for individual Jira issues.
//!
//! Combines structured metadata tables, ADF description rendering, and comments.

use crate::adf::to_terminal::adf_to_terminal;
use crate::client::issue::Issue;
use crate::ui::table::create_styled_table;
use colored::Colorize;
use comfy_table::{Attribute, Cell, Color};

/// Renders a comprehensive, multi-section view of a Jira issue for the terminal.
///
/// # Arguments
/// * `issue` - The issue details fetched from Jira.
/// * `instance_url` - The base Jira URL to build clickable links.
pub fn render_issue_detail(issue: &Issue, instance_url: &str) -> String {
    let mut out = String::new();

    // 1. Header Banner
    let issue_url = format!(
        "{}/browse/{}",
        instance_url.trim_end_matches('/'),
        issue.key
    );
    out.push_str(&format!(
        "\n{} {}\n",
        issue.key.bold().cyan(),
        format!("({})", issue_url).dimmed()
    ));
    out.push_str(&format!("{}\n\n", issue.fields.summary.bold()));

    // 2. Metadata Grid (using comfy-table for alignment)
    let mut meta_table = create_styled_table();

    let status_name = issue
        .fields
        .status
        .as_ref()
        .map(|s| s.name.as_str())
        .unwrap_or("-");
    let type_name = issue
        .fields
        .issuetype
        .as_ref()
        .map(|t| t.name.as_str())
        .unwrap_or("-");
    let priority_name = issue
        .fields
        .priority
        .as_ref()
        .map(|p| p.name.as_str())
        .unwrap_or("-");
    let assignee_name = issue
        .fields
        .assignee
        .as_ref()
        .map(|u| u.display_name.as_str())
        .unwrap_or("Unassigned");
    let reporter_name = issue
        .fields
        .reporter
        .as_ref()
        .map(|u| u.display_name.as_str())
        .unwrap_or("-");
    let labels_str = if issue.fields.labels.is_empty() {
        "-".to_string()
    } else {
        issue.fields.labels.join(", ")
    };

    meta_table.add_row(vec![
        Cell::new("Status").add_attribute(Attribute::Bold),
        Cell::new(status_name).fg(Color::Yellow),
        Cell::new("Assignee").add_attribute(Attribute::Bold),
        Cell::new(assignee_name),
    ]);

    meta_table.add_row(vec![
        Cell::new("Type").add_attribute(Attribute::Bold),
        Cell::new(type_name),
        Cell::new("Reporter").add_attribute(Attribute::Bold),
        Cell::new(reporter_name),
    ]);

    meta_table.add_row(vec![
        Cell::new("Priority").add_attribute(Attribute::Bold),
        Cell::new(priority_name).fg(Color::Cyan),
        Cell::new("Labels").add_attribute(Attribute::Bold),
        Cell::new(labels_str),
    ]);

    if let Some(created) = &issue.fields.created {
        let updated = issue.fields.updated.as_deref().unwrap_or("-");
        meta_table.add_row(vec![
            Cell::new("Created").add_attribute(Attribute::Bold),
            Cell::new(created),
            Cell::new("Updated").add_attribute(Attribute::Bold),
            Cell::new(updated),
        ]);
    }

    out.push_str(&meta_table.to_string());
    out.push_str("\n\n");

    // 3. Description Section (ADF)
    out.push_str(&format!(
        "{}\n",
        "─── Description ─────────────────────────".dimmed()
    ));
    if let Some(desc) = &issue.fields.description {
        if desc.is_empty() {
            out.push_str(&format!(
                "{}\n\n",
                "No description provided.".italic().dimmed()
            ));
        } else {
            let rendered = adf_to_terminal(desc, true);
            out.push_str(&format!("{}\n\n", rendered));
        }
    } else {
        out.push_str(&format!(
            "{}\n\n",
            "No description provided.".italic().dimmed()
        ));
    }

    // Linked Issues
    if let Some(links) = &issue.fields.issuelinks {
        if !links.is_empty() {
            out.push_str(&format!(
                "{}\n",
                format!("─── Linked Issues ({}) ───────────────────", links.len()).dimmed()
            ));
            for link in links {
                let (rel, target) = if let Some(ref o) = link.outward_issue {
                    (&link.link_type.outward, Some(o))
                } else if let Some(ref i) = link.inward_issue {
                    (&link.link_type.inward, Some(i))
                } else {
                    (&link.link_type.name, None)
                };
                let target_key = target.map(|t| t.key.as_str()).unwrap_or("-");
                let summary = target
                    .and_then(|t| t.fields.as_ref())
                    .map(|f| f.summary.as_str())
                    .unwrap_or("-");
                out.push_str(&format!(
                    "• {} {} - {}\n",
                    rel.dimmed(),
                    target_key.bold().cyan(),
                    summary
                ));
            }
            out.push('\n');
        }
    }

    // Attachments
    if let Some(attachments) = &issue.fields.attachment {
        if !attachments.is_empty() {
            out.push_str(&format!(
                "{}\n",
                format!(
                    "─── Attachments ({}) ─────────────────────",
                    attachments.len()
                )
                .dimmed()
            ));
            for att in attachments {
                out.push_str(&format!(
                    "• {} ({}, id: {})\n",
                    att.filename.bold(),
                    att.size,
                    att.id.dimmed()
                ));
            }
            out.push('\n');
        }
    }

    // 4. Comments Section (ADF)
    if let Some(comment_resp) = &issue.fields.comment {
        let count = comment_resp.comments.len();
        out.push_str(&format!(
            "{}\n",
            format!("─── Comments ({}) ─────────────────────────", count).dimmed()
        ));

        if comment_resp.comments.is_empty() {
            out.push_str(&format!("{}\n\n", "No comments yet.".italic().dimmed()));
        } else {
            for comment in &comment_resp.comments {
                let author = comment
                    .author
                    .as_ref()
                    .map(|u| u.display_name.as_str())
                    .unwrap_or("Anonymous");
                let created = comment.created.as_deref().unwrap_or("");

                out.push_str(&format!("{} {}\n", author.bold().blue(), created.dimmed()));

                if let Some(body) = &comment.body {
                    let body_text = adf_to_terminal(body, true);
                    out.push_str(&format!("{}\n\n", body_text));
                } else {
                    out.push('\n');
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::issue::IssueFields;

    #[test]
    fn test_render_issue_detail_smoke() {
        let issue = Issue {
            id: "10".to_string(),
            key: "PROJ-10".to_string(),
            self_url: None,
            fields: IssueFields {
                summary: "Smoke test issue".to_string(),
                status: None,
                issuetype: None,
                priority: None,
                assignee: None,
                reporter: None,
                created: None,
                updated: None,
                description: None,
                comment: None,
                attachment: None,
                issuelinks: None,
                labels: vec![],
            },
        };

        let rendered = render_issue_detail(&issue, "https://jira.example.com");
        assert!(rendered.contains("PROJ-10"));
        assert!(rendered.contains("Smoke test issue"));
        assert!(rendered.contains("Description"));
    }
}
