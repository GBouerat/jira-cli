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

    /// Attached files.
    pub attachment: Option<Vec<Attachment>>,

    /// Associated issue relationships.
    pub issuelinks: Option<Vec<IssueLink>>,
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

/// An attached file associated with a Jira issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    /// Attachment ID.
    pub id: String,

    /// Original file name.
    pub filename: String,

    /// Size of the file in bytes.
    pub size: u64,

    /// MIME content type (e.g., `image/png`, `application/pdf`).
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,

    /// User who uploaded the attachment.
    pub author: Option<User>,

    /// ISO-8601 upload timestamp.
    pub created: Option<String>,

    /// Direct download URL for the attachment content.
    pub content: String,
}

/// Type definition for issue links (e.g. "Blocks", "Relates to", "Clones").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueLinkType {
    /// Unique link type ID.
    pub id: String,

    /// Name of the link type (e.g. "Blocks").
    pub name: String,

    /// Inward label (e.g. "is blocked by").
    pub inward: String,

    /// Outward label (e.g. "blocks").
    pub outward: String,
}

/// Response wrapper for listing issue link types.
#[derive(Debug, Clone, Deserialize)]
pub struct IssueLinkTypesResponse {
    /// Available link types.
    #[serde(rename = "issueLinkTypes", default)]
    pub issue_link_types: Vec<IssueLinkType>,
}

/// Relationship link connecting two Jira issues.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueLink {
    /// Unique link ID.
    pub id: String,

    /// Link type definition.
    #[serde(rename = "type")]
    pub link_type: IssueLinkType,

    /// Inward linked issue if current issue is the outward side.
    #[serde(rename = "inwardIssue")]
    pub inward_issue: Option<LinkedIssue>,

    /// Outward linked issue if current issue is the inward side.
    #[serde(rename = "outwardIssue")]
    pub outward_issue: Option<LinkedIssue>,
}

/// Minimal representation of an issue referenced inside a link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedIssue {
    /// Issue ID.
    pub id: String,

    /// Issue key (e.g. "PROJ-10").
    pub key: String,

    /// Summary and status fields of the linked issue.
    pub fields: Option<LinkedIssueFields>,
}

/// Summary fields of a linked issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedIssueFields {
    /// Summary of the linked issue.
    pub summary: String,

    /// Status of the linked issue.
    pub status: Option<Status>,

    /// Priority of the linked issue.
    pub priority: Option<Priority>,

    /// Type of the linked issue.
    pub issuetype: Option<IssueType>,
}

/// Watcher status and list for an issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Watchers {
    /// Self URL.
    #[serde(rename = "self")]
    pub self_url: String,

    /// Whether the authenticated user is watching this issue.
    #[serde(rename = "isWatching")]
    pub is_watching: bool,

    /// Total count of watchers.
    #[serde(rename = "watchCount")]
    pub watch_count: u64,

    /// List of watching users.
    #[serde(default)]
    pub watchers: Vec<User>,
}

/// History record for field modifications on an issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Changelog {
    /// Changelog record ID.
    pub id: String,

    /// User who made the change.
    pub author: Option<User>,

    /// ISO-8601 timestamp when the change was made.
    pub created: String,

    /// Individual field changes.
    #[serde(default)]
    pub items: Vec<ChangeItem>,
}

/// Individual field modification inside a changelog entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeItem {
    /// Name of the field modified (e.g. "status", "assignee", "summary").
    pub field: String,

    /// Previous value display string.
    #[serde(rename = "fromString")]
    pub from_string: Option<String>,

    /// New value display string.
    #[serde(rename = "toString")]
    pub to_string: Option<String>,
}

/// Paginated changelog response.
#[derive(Debug, Clone, Deserialize)]
pub struct ChangelogResponse {
    /// List of history records.
    #[serde(default)]
    pub values: Vec<Changelog>,

    /// Total count of history records.
    #[serde(default)]
    pub total: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_attachment() {
        let json = r#"{
            "id": "10050",
            "filename": "screenshot.png",
            "size": 102400,
            "mimeType": "image/png",
            "content": "https://example.atlassian.net/rest/api/3/attachment/content/10050"
        }"#;

        let att: Attachment = serde_json::from_str(json).unwrap();
        assert_eq!(att.filename, "screenshot.png");
        assert_eq!(att.size, 102400);
        assert_eq!(att.mime_type.as_deref(), Some("image/png"));
    }

    #[test]
    fn test_deserialize_issue_link() {
        let json = r#"{
            "id": "100",
            "type": {
                "id": "1",
                "name": "Blocks",
                "inward": "is blocked by",
                "outward": "blocks"
            },
            "outwardIssue": {
                "id": "1002",
                "key": "DEV-2",
                "fields": {
                    "summary": "Target blocked issue"
                }
            }
        }"#;

        let link: IssueLink = serde_json::from_str(json).unwrap();
        assert_eq!(link.link_type.name, "Blocks");
        assert_eq!(link.outward_issue.unwrap().key, "DEV-2");
    }

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
