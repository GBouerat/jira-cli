//! Terminal table rendering with `comfy-table`.
//!
//! Enforces zero raw ANSI strings inside table cells to preserve exact column
//! width calculations and perfect border alignment. Styling is applied exclusively
//! via [`comfy_table::Cell::fg`] and [`comfy_table::Cell::add_attribute`].

use crate::client::issue::{Issue, Transition};
use crate::client::project::Project;
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
        Cell::new("KEY").add_attribute(Attribute::Bold).fg(Color::Cyan),
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
                if status_name.eq_ignore_ascii_case("done") || status_name.eq_ignore_ascii_case("closed") {
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
        Cell::new("KEY").add_attribute(Attribute::Bold).fg(Color::Cyan),
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
        Cell::new("ID").add_attribute(Attribute::Bold).fg(Color::Cyan),
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
        Cell::new("ID").add_attribute(Attribute::Bold).fg(Color::Cyan),
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
        Cell::new("ID").add_attribute(Attribute::Bold).fg(Color::Cyan),
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
                labels: vec![],
            },
        };

        let table_str = render_issues_table(&[issue]);
        assert!(table_str.contains("DEV-1"));
        assert!(table_str.contains("Test issue"));
        assert!(table_str.contains("Open"));
    }
}
