//! Jira Cloud Issue API models and endpoints.
//!
//! Covers issue queries (JQL), issue details, creation, updates, workflow transitions,
//! comments, and worklog logging using Jira REST API v3 with ADF payloads.

use crate::adf::ast::DocNode;
use serde::{Deserialize, Serialize};

/// An individual Jira issue representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    /// Internal Jira issue ID.
    pub id: String,

    /// Issue key (e.g., `PROJ-123`).
    pub key: String,

    /// Self-referential URL to the issue resource.
    #[serde(rename = "self")]
    pub self_url: Option<String>,

    /// Detailed issue fields.
    pub fields: IssueFields,
}

/// The collection of standard fields associated with a Jira issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IssueFields {
    /// One-line issue summary or title.
    pub summary: String,

    /// Workflow status of the issue.
    pub status: Option<Status>,

    /// Issue type (e.g., Bug, Task, Story).
    pub issuetype: Option<IssueType>,

    /// Issue priority (e.g., High, Medium, Low).
    pub priority: Option<Priority>,

    /// Assigned user.
    pub assignee: Option<User>,

    /// Reporting user.
    pub reporter: Option<User>,

    /// ISO-8601 creation timestamp.
    pub created: Option<String>,

    /// ISO-8601 last updated timestamp.
    pub updated: Option<String>,

    /// Rich-text description stored in Atlassian Document Format (ADF).
    pub description: Option<DocNode>,

    /// Embedded comments collection.
    pub comment: Option<CommentsResponse>,

    /// Associated issue labels.
    #[serde(default)]
    pub labels: Vec<String>,
}

/// Workflow status of an issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// Status ID.
    pub id: String,

    /// Status display name (e.g., "In Progress", "Done").
    pub name: String,

    /// High-level status category.
    #[serde(rename = "statusCategory")]
    pub status_category: Option<StatusCategory>,
}

/// High-level status category grouping (e.g., "To Do", "In Progress", "Done").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusCategory {
    /// Category ID.
    pub id: u64,

    /// Category key (e.g., "new", "indeterminate", "done").
    pub key: String,

    /// Category display name.
    pub name: String,
}

/// Type definition for an issue (e.g., "Bug", "Story", "Task").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueType {
    /// Issue type ID.
    pub id: String,

    /// Display name of the issue type.
    pub name: String,

    /// Whether this type is a subtask.
    #[serde(default)]
    pub subtask: bool,
}

/// Priority level of an issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Priority {
    /// Priority ID.
    pub id: String,

    /// Priority display name (e.g., "High", "Medium", "Low").
    pub name: String,
}

/// User details for assignees, reporters, authors, and commenters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// Unique Atlassian account ID.
    #[serde(rename = "accountId")]
    pub account_id: String,

    /// Display name of the user.
    #[serde(rename = "displayName")]
    pub display_name: String,

    /// User email address (if visible based on privacy settings).
    #[serde(rename = "emailAddress")]
    pub email_address: Option<String>,

    /// Whether the user account is active.
    #[serde(default = "default_true")]
    pub active: bool,
}

fn default_true() -> bool {
    true
}

/// Paginated collection of comments returned by the issue resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CommentsResponse {
    /// List of comments.
    #[serde(default)]
    pub comments: Vec<Comment>,

    /// Total count of comments on the issue.
    #[serde(default)]
    pub total: u64,

    /// Starting index offset.
    #[serde(rename = "startAt", default)]
    pub start_at: u64,

    /// Maximum results returned in this page.
    #[serde(rename = "maxResults", default)]
    pub max_results: u64,
}

/// An individual comment on a Jira issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    /// Unique comment ID.
    pub id: String,

    /// Author who posted the comment.
    pub author: Option<User>,

    /// Rich-text comment body in Atlassian Document Format (ADF).
    pub body: Option<DocNode>,

    /// ISO-8601 creation timestamp.
    pub created: Option<String>,

    /// ISO-8601 last update timestamp.
    pub updated: Option<String>,
}

/// Available workflow transition for an issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transition {
    /// Unique transition ID needed to perform the transition.
    pub id: String,

    /// Display name of the transition (e.g., "Start Progress", "Resolve Issue").
    pub name: String,

    /// Target destination status after transition.
    pub to: Status,
}

/// Response wrapper for listing transitions.
#[derive(Debug, Clone, Deserialize)]
pub struct TransitionsResponse {
    /// Collection of available transitions.
    #[serde(default)]
    pub transitions: Vec<Transition>,
}

/// Search results returned by `/rest/api/3/search`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResults {
    /// Zero-based index of the first item returned.
    #[serde(rename = "startAt")]
    pub start_at: u64,

    /// Maximum number of items requested/returned.
    #[serde(rename = "maxResults")]
    pub max_results: u64,

    /// Total number of issues matching the JQL query.
    pub total: u64,

    /// Matching issue objects.
    #[serde(default)]
    pub issues: Vec<Issue>,
}

/// Response returned after successfully creating an issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatedIssue {
    /// Generated issue ID.
    pub id: String,

    /// Generated issue key (e.g., `PROJ-101`).
    pub key: String,

    /// Self-referential URL.
    #[serde(rename = "self")]
    pub self_url: String,
}

/// Worklog entry record for an issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Worklog {
    /// Worklog ID.
    pub id: String,

    /// User who logged the work.
    pub author: Option<User>,

    /// Human-readable time spent (e.g., "3h 20m").
    #[serde(rename = "timeSpent")]
    pub time_spent: String,

    /// Time spent in seconds.
    #[serde(rename = "timeSpentSeconds")]
    pub time_spent_seconds: u64,

    /// Optional comment describing the logged work in ADF format.
    pub comment: Option<DocNode>,

    /// ISO-8601 start timestamp.
    pub started: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_issue_json() {
        let json = r#"{
            "id": "10001",
            "key": "DEV-42",
            "self": "https://example.atlassian.net/rest/api/3/issue/10001",
            "fields": {
                "summary": "Fix login crash",
                "status": {
                    "id": "3",
                    "name": "In Progress",
                    "statusCategory": {
                        "id": 4,
                        "key": "indeterminate",
                        "name": "In Progress"
                    }
                },
                "issuetype": {
                    "id": "1",
                    "name": "Bug",
                    "subtask": false
                },
                "labels": ["backend", "urgent"]
            }
        }"#;

        let issue: Issue = serde_json::from_str(json).expect("failed to deserialize issue");
        assert_eq!(issue.key, "DEV-42");
        assert_eq!(issue.fields.summary, "Fix login crash");
        assert_eq!(issue.fields.status.unwrap().name, "In Progress");
        assert_eq!(issue.fields.labels, vec!["backend", "urgent"]);
    }
}
