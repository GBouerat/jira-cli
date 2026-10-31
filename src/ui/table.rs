//! Terminal table rendering with `comfy-table`.
//!
//! Enforces zero raw ANSI strings inside table cells to preserve exact column
//! width calculations and perfect border alignment. Styling is applied exclusively
//! via [`comfy_table::Cell::fg`] and [`comfy_table::Cell::add_attribute`].

use crate::client::issue::{
    Attachment, Changelog, Comment, Issue, IssueLink, Transition, Watchers,
};
use crate::client::project::{Component, Project, Version};
use crate::client::sprint::{Board, Sprint};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};

/// Builds a styled table initialized with rounded UTF-8 borders and dynamic arrangement.
pub fn create_styled_table() -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}

/// Renders a list of Jira issues into a terminal table.
pub fn render_issues_table(issues: &[Issue]) -> String {
    if issues.is_empty() {
        return "No issues found matching query.".to_string();
    }

    let mut table = create_styled_table();
    table.set_header(vec![
        Cell::new("KEY")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("TYPE").add_attribute(Attribute::Bold),
        Cell::new("SUMMARY").add_attribute(Attribute::Bold),
        Cell::new("STATUS").add_attribute(Attribute::Bold),
        Cell::new("PRIORITY").add_attribute(Attribute::Bold),
        Cell::new("ASSIGNEE").add_attribute(Attribute::Bold),
    ]);

    for issue in issues {
        let key_cell = Cell::new(&issue.key)
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold);

        let type_name = issue
            .fields
            .issuetype
            .as_ref()
            .map(|t| t.name.as_str())
            .unwrap_or("-");
        let type_cell = Cell::new(type_name);

        let summary_cell = Cell::new(&issue.fields.summary);

        let status_name = issue
            .fields
            .status
            .as_ref()
            .map(|s| s.name.as_str())
            .unwrap_or("-");
        let status_color = match issue
            .fields
            .status
            .as_ref()
            .and_then(|s| s.status_category.as_ref())
            .map(|sc| sc.key.as_str())
        {
            Some("done") => Color::Green,
            Some("indeterminate") => Color::Yellow,
            Some("new") => Color::Blue,
            _ => {
                if status_name.eq_ignore_ascii_case("done")
                    || status_name.eq_ignore_ascii_case("closed")
                {
                    Color::Green
                } else if status_name.eq_ignore_ascii_case("in progress") {
                    Color::Yellow
                } else {
                    Color::White
                }
            }
        };
        let status_cell = Cell::new(status_name).fg(status_color);

        let priority_name = issue
            .fields
            .priority
            .as_ref()
            .map(|p| p.name.as_str())
            .unwrap_or("-");
        let priority_color = match priority_name.to_ascii_lowercase().as_str() {
            "highest" | "high" | "critical" | "blocker" => Color::Red,
            "medium" => Color::Yellow,
            "low" | "lowest" => Color::Green,
            _ => Color::White,
        };
        let priority_cell = Cell::new(priority_name).fg(priority_color);

        let assignee_name = issue
            .fields
            .assignee
            .as_ref()
            .map(|u| u.display_name.as_str())
            .unwrap_or("Unassigned");
        let assignee_cell = if assignee_name == "Unassigned" {
            Cell::new(assignee_name).fg(Color::DarkGrey)
        } else {
            Cell::new(assignee_name)
        };

        table.add_row(vec![
            key_cell,
            type_cell,
            summary_cell,
            status_cell,
            priority_cell,
            assignee_cell,
        ]);
    }

    table.to_string()
}

/// Renders a list of Jira projects into a terminal table.
pub fn render_projects_table(projects: &[Project]) -> String {
    if projects.is_empty() {
        return "No projects found.".to_string();
    }

    let mut table = create_styled_table();
    table.set_header(vec![
        Cell::new("KEY")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("NAME").add_attribute(Attribute::Bold),
        Cell::new("TYPE").add_attribute(Attribute::Bold),
        Cell::new("LEAD").add_attribute(Attribute::Bold),
    ]);

    for project in projects {
        let key_cell = Cell::new(&project.key)
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold);
        let name_cell = Cell::new(&project.name);
        let type_cell = Cell::new(project.project_type_key.as_deref().unwrap_or("-"));
        let lead_cell = Cell::new(
            project
                .lead
                .as_ref()
                .map(|u| u.display_name.as_str())
                .unwrap_or("-"),
        );

        table.add_row(vec![key_cell, name_cell, type_cell, lead_cell]);
    }

    table.to_string()
}

/// Renders a list of Agile sprints into a terminal table.
pub fn render_sprints_table(sprints: &[Sprint]) -> String {
    if sprints.is_empty() {
        return "No sprints found.".to_string();
    }

    let mut table = create_styled_table();
    table.set_header(vec![
        Cell::new("ID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("NAME").add_attribute(Attribute::Bold),
        Cell::new("STATE").add_attribute(Attribute::Bold),
        Cell::new("START DATE").add_attribute(Attribute::Bold),
        Cell::new("END DATE").add_attribute(Attribute::Bold),
    ]);

    for sprint in sprints {
        let id_cell = Cell::new(sprint.id.to_string())
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold);
        let name_cell = Cell::new(&sprint.name);

        let state_color = match sprint.state.to_ascii_lowercase().as_str() {
            "active" => Color::Green,
            "future" => Color::Blue,
            "closed" => Color::DarkGrey,
            _ => Color::White,
        };
        let state_cell = Cell::new(&sprint.state).fg(state_color);

        let start_cell = Cell::new(sprint.start_date.as_deref().unwrap_or("-"));
        let end_cell = Cell::new(sprint.end_date.as_deref().unwrap_or("-"));

        table.add_row(vec![id_cell, name_cell, state_cell, start_cell, end_cell]);
    }

    table.to_string()
}

/// Renders a list of Agile boards into a terminal table.
pub fn render_boards_table(boards: &[Board]) -> String {
    if boards.is_empty() {
        return "No boards found.".to_string();
    }

    let mut table = create_styled_table();
    table.set_header(vec![
        Cell::new("ID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("NAME").add_attribute(Attribute::Bold),
        Cell::new("TYPE").add_attribute(Attribute::Bold),
    ]);

    for board in boards {
        let id_cell = Cell::new(board.id.to_string())
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold);
        let name_cell = Cell::new(&board.name);
        let type_cell = Cell::new(&board.board_type);

        table.add_row(vec![id_cell, name_cell, type_cell]);
    }

    table.to_string()
}

/// Renders a list of workflow transitions into a terminal table.
pub fn render_transitions_table(transitions: &[Transition]) -> String {
    if transitions.is_empty() {
        return "No transitions available for this issue.".to_string();
    }

    let mut table = create_styled_table();
    table.set_header(vec![
        Cell::new("ID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("TRANSITION NAME").add_attribute(Attribute::Bold),
        Cell::new("TARGET STATUS").add_attribute(Attribute::Bold),
    ]);

    for trans in transitions {
        let id_cell = Cell::new(&trans.id)
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold);
        let name_cell = Cell::new(&trans.name);
        let to_cell = Cell::new(&trans.to.name).fg(Color::Green);

        table.add_row(vec![id_cell, name_cell, to_cell]);
    }

    table.to_string()
}

/// Formats a raw byte count into human-readable units (B, KB, MB, GB).
pub fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Renders a list of issue attachments into a terminal table.
pub fn render_attachments_table(attachments: &[Attachment]) -> String {
    if attachments.is_empty() {
        return "No attachments found for this issue.".to_string();
    }

    let mut table = create_styled_table();
    table.set_header(vec![
        Cell::new("ID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("FILENAME").add_attribute(Attribute::Bold),
        Cell::new("SIZE").add_attribute(Attribute::Bold),
        Cell::new("MIME TYPE").add_attribute(Attribute::Bold),
        Cell::new("AUTHOR").add_attribute(Attribute::Bold),
        Cell::new("CREATED").add_attribute(Attribute::Bold),
    ]);

    for att in attachments {
        let id_cell = Cell::new(&att.id)
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold);
        let name_cell = Cell::new(&att.filename).add_attribute(Attribute::Bold);
        let size_cell = Cell::new(format_file_size(att.size));
        let mime_cell = Cell::new(att.mime_type.as_deref().unwrap_or("-"));
        let author_cell = Cell::new(
            att.author
                .as_ref()
                .map(|u| u.display_name.as_str())
                .unwrap_or("-"),
        );
        let created_cell = Cell::new(att.created.as_deref().unwrap_or("-"));

        table.add_row(vec![
            id_cell,
            name_cell,
            size_cell,
            mime_cell,
            author_cell,
            created_cell,
        ]);
    }

    table.to_string()
}

/// Renders a list of issue links into a terminal table.
pub fn render_issue_links_table(links: &[IssueLink], current_key: &str) -> String {
    if links.is_empty() {
        return "No linked issues found.".to_string();
    }

    let mut table = create_styled_table();
    table.set_header(vec![
        Cell::new("LINK ID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("RELATIONSHIP").add_attribute(Attribute::Bold),
        Cell::new("KEY")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("SUMMARY").add_attribute(Attribute::Bold),
        Cell::new("STATUS").add_attribute(Attribute::Bold),
    ]);

    let upper_current = current_key.to_uppercase();

    for link in links {
        let id_cell = Cell::new(&link.id)
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold);

        let (rel, target_opt) = if let Some(ref out) = link.outward_issue {
            if out.key.to_uppercase() != upper_current {
                (&link.link_type.outward, Some(out))
            } else if let Some(ref inw) = link.inward_issue {
                (&link.link_type.inward, Some(inw))
            } else {
                (&link.link_type.outward, Some(out))
            }
        } else if let Some(ref inw) = link.inward_issue {
            (&link.link_type.inward, Some(inw))
        } else {
            (&link.link_type.name, None)
        };

        let rel_cell = Cell::new(rel);

        let (key_cell, summary_cell, status_cell) = match target_opt {
            Some(t) => {
                let kc = Cell::new(&t.key)
                    .fg(Color::Cyan)
                    .add_attribute(Attribute::Bold);
                let sum = t.fields.as_ref().map(|f| f.summary.as_str()).unwrap_or("-");
                let sc = Cell::new(sum);
                let st_name = t
                    .fields
                    .as_ref()
                    .and_then(|f| f.status.as_ref())
                    .map(|s| s.name.as_str())
                    .unwrap_or("-");
                let stc = Cell::new(st_name).fg(Color::Green);
                (kc, sc, stc)
            }
            None => (Cell::new("-"), Cell::new("-"), Cell::new("-")),
        };

        table.add_row(vec![id_cell, rel_cell, key_cell, summary_cell, status_cell]);
    }

    table.to_string()
}

/// Renders a list of watchers into a terminal table.
pub fn render_watchers_table(watchers: &Watchers) -> String {
    if watchers.watchers.is_empty() {
        return format!(
            "No watchers registered (total watch count: {}).",
            watchers.watch_count
        );
    }

    let mut table = create_styled_table();
    table.set_header(vec![
        Cell::new("ACCOUNT ID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("DISPLAY NAME").add_attribute(Attribute::Bold),
        Cell::new("ACTIVE").add_attribute(Attribute::Bold),
    ]);

    for user in &watchers.watchers {
        let id_cell = Cell::new(&user.account_id)
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold);
        let name_cell = Cell::new(&user.display_name);
        let active_cell = if user.active {
            Cell::new("yes").fg(Color::Green)
        } else {
            Cell::new("no").fg(Color::DarkGrey)
        };

        table.add_row(vec![id_cell, name_cell, active_cell]);
    }

    table.to_string()
}

/// Renders a list of comments into a formatted terminal table including comment IDs.
pub fn render_comments_table(comments: &[Comment]) -> String {
    if comments.is_empty() {
        return "No comments found on this issue.".to_string();
    }

    let mut table = create_styled_table();
    table.set_header(vec![
        Cell::new("ID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("AUTHOR").add_attribute(Attribute::Bold),
        Cell::new("CREATED").add_attribute(Attribute::Bold),
        Cell::new("BODY").add_attribute(Attribute::Bold),
    ]);

    for comment in comments {
        let id_cell = Cell::new(&comment.id)
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold);

        let author_name = comment
            .author
            .as_ref()
            .map(|u| u.display_name.as_str())
            .unwrap_or("Anonymous");
        let author_cell = Cell::new(author_name);

        let created_str = comment.created.as_deref().unwrap_or("-");
        let created_cell = Cell::new(created_str);

        let body_preview = if let Some(ref body) = comment.body {
            let rendered = crate::adf::adf_to_terminal(body, false);
            let first_line = rendered.lines().next().unwrap_or("").trim();
            if first_line.len() > 60 {
                format!("{}...", &first_line[..57])
            } else if first_line.is_empty() {
                "-".to_string()
            } else {
                first_line.to_string()
            }
        } else {
            "-".to_string()
        };
        let body_cell = Cell::new(body_preview);

        table.add_row(vec![id_cell, author_cell, created_cell, body_cell]);
    }

    table.to_string()
}

/// Renders changelog history records into a terminal table.
pub fn render_changelog_table(changelogs: &[Changelog]) -> String {
    if changelogs.is_empty() {
        return "No field history records found.".to_string();
    }

    let mut table = create_styled_table();
    table.set_header(vec![
        Cell::new("DATE")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("AUTHOR").add_attribute(Attribute::Bold),
        Cell::new("FIELD").add_attribute(Attribute::Bold),
        Cell::new("FROM").add_attribute(Attribute::Bold),
        Cell::new("TO").add_attribute(Attribute::Bold),
    ]);

    for log in changelogs {
        let date_str = &log.created;
        let author_name = log
            .author
            .as_ref()
            .map(|u| u.display_name.as_str())
            .unwrap_or("System");

        for item in &log.items {
            let date_cell = Cell::new(date_str)
                .fg(Color::Cyan)
                .add_attribute(Attribute::Bold);
            let author_cell = Cell::new(author_name);
            let field_cell = Cell::new(&item.field).add_attribute(Attribute::Bold);
            let from_cell = Cell::new(item.from_string.as_deref().unwrap_or("-"));
            let to_cell = Cell::new(item.to_string.as_deref().unwrap_or("-")).fg(Color::Green);

            table.add_row(vec![date_cell, author_cell, field_cell, from_cell, to_cell]);
        }
    }

    table.to_string()
}

/// Renders a list of release versions into a terminal table.
pub fn render_versions_table(versions: &[Version]) -> String {
    if versions.is_empty() {
        return "No versions found for this project.".to_string();
    }

    let mut table = create_styled_table();
    table.set_header(vec![
        Cell::new("ID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("NAME").add_attribute(Attribute::Bold),
        Cell::new("RELEASED").add_attribute(Attribute::Bold),
        Cell::new("RELEASE DATE").add_attribute(Attribute::Bold),
        Cell::new("ARCHIVED").add_attribute(Attribute::Bold),
        Cell::new("DESCRIPTION").add_attribute(Attribute::Bold),
    ]);

    for ver in versions {
        let id_cell = Cell::new(&ver.id)
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold);
        let name_cell = Cell::new(&ver.name).add_attribute(Attribute::Bold);
        let released_cell = if ver.released {
            Cell::new("yes").fg(Color::Green)
        } else {
            Cell::new("no").fg(Color::Yellow)
        };
        let date_cell = Cell::new(ver.release_date.as_deref().unwrap_or("-"));
        let archived_cell = if ver.archived {
            Cell::new("yes").fg(Color::DarkGrey)
        } else {
            Cell::new("no")
        };
        let desc_cell = Cell::new(ver.description.as_deref().unwrap_or("-"));

        table.add_row(vec![
            id_cell,
            name_cell,
            released_cell,
            date_cell,
            archived_cell,
            desc_cell,
        ]);
    }

    table.to_string()
}

/// Renders a list of project components into a terminal table.
pub fn render_components_table(components: &[Component]) -> String {
    if components.is_empty() {
        return "No components found for this project.".to_string();
    }

    let mut table = create_styled_table();
    table.set_header(vec![
        Cell::new("ID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("NAME").add_attribute(Attribute::Bold),
        Cell::new("LEAD").add_attribute(Attribute::Bold),
        Cell::new("DESCRIPTION").add_attribute(Attribute::Bold),
    ]);

    for comp in components {
        let id_cell = Cell::new(&comp.id)
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold);
        let name_cell = Cell::new(&comp.name).add_attribute(Attribute::Bold);
        let lead_cell = Cell::new(
            comp.lead
                .as_ref()
                .map(|u| u.display_name.as_str())
                .unwrap_or("-"),
        );
        let desc_cell = Cell::new(comp.description.as_deref().unwrap_or("-"));

        table.add_row(vec![id_cell, name_cell, lead_cell, desc_cell]);
    }

    table.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::issue::{IssueFields, IssueType, Priority, Status};

    #[test]
    fn test_render_issues_table_styling() {
        let issue = Issue {
            id: "100".to_string(),
            key: "DEV-1".to_string(),
            self_url: None,
            fields: IssueFields {
                summary: "Test issue".to_string(),
                status: Some(Status {
                    id: "1".to_string(),
                    name: "Open".to_string(),
                    status_category: None,
                }),
                issuetype: Some(IssueType {
                    id: "1".to_string(),
                    name: "Task".to_string(),
                    subtask: false,
                }),
                priority: Some(Priority {
                    id: "2".to_string(),
                    name: "High".to_string(),
                }),
                assignee: None,
                reporter: None,
                created: None,
                updated: None,
                description: None,
                comment: None,
                attachment: None,
                issuelinks: None,
                project: None,
                subtasks: None,
                parent: None,
                labels: vec![],
            },
        };

        let table_str = render_issues_table(&[issue]);
        assert!(table_str.contains("DEV-1"));
        assert!(table_str.contains("Test issue"));
        assert!(table_str.contains("Open"));
    }

    #[test]
    fn test_render_attachments_table() {
        let att = Attachment {
            id: "101".to_string(),
            filename: "log.txt".to_string(),
            size: 2048,
            mime_type: Some("text/plain".to_string()),
            author: None,
            created: Some("2026-10-30".to_string()),
            content: "https://example.com/log.txt".to_string(),
        };
        let table = render_attachments_table(&[att]);
        assert!(table.contains("101"));
        assert!(table.contains("log.txt"));
        assert!(table.contains("2.0 KB"));
    }

    #[test]
    fn test_render_issue_links_table() {
        let link = IssueLink {
            id: "501".to_string(),
            link_type: crate::client::issue::IssueLinkType {
                id: "1".to_string(),
                name: "Blocks".to_string(),
                inward: "is blocked by".to_string(),
                outward: "blocks".to_string(),
            },
            inward_issue: None,
            outward_issue: Some(crate::client::issue::LinkedIssue {
                id: "1002".to_string(),
                key: "DEV-2".to_string(),
                fields: Some(crate::client::issue::LinkedIssueFields {
                    summary: "Downstream task".to_string(),
                    status: None,
                    priority: None,
                    issuetype: None,
                }),
            }),
        };
        let table = render_issue_links_table(&[link], "DEV-1");
        assert!(table.contains("501"));
        assert!(table.contains("blocks"));
        assert!(table.contains("DEV-2"));
        assert!(table.contains("Downstream task"));
    }

    #[test]
    fn test_render_versions_and_components_table() {
        let ver = Version {
            id: "1".to_string(),
            name: "v1.0.0".to_string(),
            description: Some("Initial release".to_string()),
            archived: false,
            released: true,
            release_date: Some("2026-10-30".to_string()),
        };
        let table_v = render_versions_table(&[ver]);
        assert!(table_v.contains("v1.0.0"));
        assert!(table_v.contains("yes"));

        let comp = Component {
            id: "10".to_string(),
            name: "Backend".to_string(),
            description: Some("Core API".to_string()),
            lead: None,
        };
        let table_c = render_components_table(&[comp]);
        assert!(table_c.contains("Backend"));
        assert!(table_c.contains("Core API"));
    }
}
