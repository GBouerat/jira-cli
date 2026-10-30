//! Jira Software Agile API models for boards and sprints.
//!
//! Covers Agile board discovery, sprint listing, and sprint issue queries.

use serde::{Deserialize, Serialize};

/// An Agile Scrum or Kanban board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Board {
    /// Unique board ID.
    pub id: u64,

    /// Board display name.
    pub name: String,

    /// Type of board (e.g., `"scrum"`, `"kanban"`).
    #[serde(rename = "type")]
    pub board_type: String,
}

/// Paginated response from `/rest/agile/1.0/board`.
#[derive(Debug, Clone, Deserialize)]
pub struct BoardsResponse {
    /// Collection of boards.
    #[serde(default)]
    pub values: Vec<Board>,

    /// Total number of boards.
    #[serde(default)]
    pub total: u64,

    /// Whether this is the last page.
    #[serde(rename = "isLast", default)]
    pub is_last: bool,
}

/// A development sprint belonging to an Agile board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sprint {
    /// Unique sprint ID.
    pub id: u64,

    /// Sprint name (e.g., "Sprint 14").
    pub name: String,

    /// State of the sprint (e.g., `"active"`, `"future"`, `"closed"`).
    pub state: String,

    /// Start date timestamp.
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,

    /// Scheduled end date timestamp.
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,

    /// Goal description set for the sprint.
    pub goal: Option<String>,
}

/// Paginated response from `/rest/agile/1.0/board/{boardId}/sprint`.
#[derive(Debug, Clone, Deserialize)]
pub struct SprintsResponse {
    /// Collection of sprints.
    #[serde(default)]
    pub values: Vec<Sprint>,

    /// Total count.
    #[serde(default)]
    pub total: u64,

    /// Whether this is the last page.
    #[serde(rename = "isLast", default)]
    pub is_last: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_sprint() {
        let json = r#"{
            "id": 42,
            "name": "Sprint 42",
            "state": "active",
            "startDate": "2026-10-01T09:00:00.000Z",
            "endDate": "2026-10-15T18:00:00.000Z",
            "goal": "Deliver CLI v1.0"
        }"#;

        let sprint: Sprint = serde_json::from_str(json).unwrap();
        assert_eq!(sprint.id, 42);
        assert_eq!(sprint.name, "Sprint 42");
        assert_eq!(sprint.state, "active");
        assert_eq!(sprint.goal.as_deref(), Some("Deliver CLI v1.0"));
    }
}
