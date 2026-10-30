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

/// Project release version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    /// Version ID.
    pub id: String,

    /// Name of the version (e.g. "v1.2.0").
    pub name: String,

    /// Optional description of the release.
    pub description: Option<String>,

    /// Whether the version is archived.
    #[serde(default)]
    pub archived: bool,

    /// Whether the version is marked as released.
    #[serde(default)]
    pub released: bool,

    /// ISO-8601 release date.
    #[serde(rename = "releaseDate")]
    pub release_date: Option<String>,
}

/// Project subsystem component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Component {
    /// Component ID.
    pub id: String,

    /// Name of the component (e.g. "Auth", "Backend", "UI").
    pub name: String,

    /// Description of the component.
    pub description: Option<String>,

    /// Component lead user.
    pub lead: Option<User>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_version() {
        let json = r#"{
            "id": "1000",
            "name": "1.0.0",
            "released": true,
            "releaseDate": "2026-10-30"
        }"#;

        let ver: Version = serde_json::from_str(json).unwrap();
        assert_eq!(ver.name, "1.0.0");
        assert!(ver.released);
    }

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
