//! Jira Cloud Project API models and endpoints.
//!
//! Provides models for Jira projects, categories, project leads, and issue type schemes.

use crate::client::issue::{IssueType, User};
use serde::{Deserialize, Serialize};

/// Project details returned by Jira Cloud REST API v3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    /// Internal project ID.
    pub id: String,

    /// Project key (e.g., `DEV`, `PROJ`).
    pub key: String,

    /// Full project name.
    pub name: String,

    /// Project type key (e.g., `software`, `business`, `service_desk`).
    #[serde(rename = "projectTypeKey")]
    pub project_type_key: Option<String>,

    /// Project lead user.
    pub lead: Option<User>,

    /// Description of the project.
    pub description: Option<String>,

    /// List of issue types available in this project.
    #[serde(rename = "issueTypes", default)]
    pub issue_types: Vec<IssueType>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_project() {
        let json = r#"{
            "id": "10000",
            "key": "CORE",
            "name": "Core Backend Engine",
            "projectTypeKey": "software",
            "description": "Main platform backend services"
        }"#;

        let proj: Project = serde_json::from_str(json).unwrap();
        assert_eq!(proj.key, "CORE");
        assert_eq!(proj.name, "Core Backend Engine");
        assert_eq!(proj.project_type_key.as_deref(), Some("software"));
    }
}
