//! Jira Cloud REST API v3 HTTP client implementation.
//!
//! Provides the primary [`JiraClient`] struct for communicating with Jira Cloud,
//! executing authenticated requests, handling Atlassian Document Format (ADF) payloads,
//! and gracefully managing errors and rate limits.

pub mod auth;
pub mod issue;
pub mod middleware;
pub mod project;
pub mod sprint;

use crate::adf::ast::DocNode;
use crate::client::auth::{build_basic_auth_header, build_user_agent_header};
use crate::client::issue::{
    Comment, CommentsResponse, CreatedIssue, Issue, SearchResults, Transition, TransitionsResponse,
    User, Worklog,
};
use crate::client::middleware::handle_response_errors;
use crate::client::project::Project;
use crate::client::sprint::{Board, BoardsResponse, Sprint, SprintsResponse};
use crate::config::ResolvedConfig;
use crate::error::{JiraError, Result};
use reqwest::{Client, Method, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::time::Duration;

/// Jira Cloud REST API v3 client.
#[derive(Debug, Clone)]
pub struct JiraClient {
    /// Validated base instance URL without trailing slash (e.g., `https://example.atlassian.net`).
    instance_url: String,

    /// HTTP client instance.
    http: Client,

    /// Encoded `Authorization: Basic ...` header.
    auth_header: String,

    /// User-Agent header value.
    user_agent: String,
}

impl JiraClient {
    /// Creates a new [`JiraClient`] from fully resolved configuration credentials.
    ///
    /// # Arguments
    /// * `config` - The resolved configuration containing URL, email, and API token.
    pub fn new(config: &ResolvedConfig) -> Result<Self> {
        Self::from_credentials(&config.instance_url, &config.user_email, &config.api_token)
    }

    /// Creates a new [`JiraClient`] from direct credential arguments.
    ///
    /// # Arguments
    /// * `instance_url` - Jira Cloud base URL.
    /// * `user_email` - User email address.
    /// * `api_token` - Atlassian API token.
    pub fn from_credentials(instance_url: &str, user_email: &str, api_token: &str) -> Result<Self> {
        let trimmed_url = instance_url.trim().trim_end_matches('/');
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(JiraError::Network)?;

        let auth_header = build_basic_auth_header(user_email, api_token);
        let user_agent = build_user_agent_header();

        Ok(Self {
            instance_url: trimmed_url.to_string(),
            http,
            auth_header,
            user_agent,
        })
    }

    /// Returns the configured Jira instance URL.
    pub fn instance_url(&self) -> &str {
        &self.instance_url
    }

    /// Builds a base HTTP request with authentication and standard headers preconfigured.
    fn request(&self, method: Method, endpoint: &str) -> RequestBuilder {
        let full_url = if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
            endpoint.to_string()
        } else {
            let path = if endpoint.starts_with('/') {
                endpoint.to_string()
            } else {
                format!("/{}", endpoint)
            };
            format!("{}{}", self.instance_url, path)
        };

        self.http
            .request(method, &full_url)
            .header(reqwest::header::AUTHORIZATION, &self.auth_header)
            .header(reqwest::header::USER_AGENT, &self.user_agent)
            .header(reqwest::header::ACCEPT, "application/json")
    }

    /// Executes an HTTP request, checks for API errors and rate limits, and returns the raw response.
    async fn execute(&self, req: RequestBuilder) -> Result<Response> {
        let resp = req.send().await.map_err(JiraError::Network)?;
        handle_response_errors(resp).await
    }

    /// Executes an HTTP request and deserializes the JSON body into type `T`.
    async fn execute_json<T: DeserializeOwned>(&self, req: RequestBuilder) -> Result<T> {
        let resp = self.execute(req).await?;
        let bytes = resp.bytes().await.map_err(JiraError::Network)?;
        serde_json::from_slice(&bytes).map_err(JiraError::Json)
    }

    // ------------------------------------------------------------------------
    // User & Session APIs
    // ------------------------------------------------------------------------

    /// Verifies authentication and returns the current user profile from `/rest/api/3/myself`.
    pub async fn myself(&self) -> Result<User> {
        let req = self.request(Method::GET, "/rest/api/3/myself");
        self.execute_json(req).await
    }

    /// Searches for active users matching a query string.
    pub async fn find_users(&self, query: &str) -> Result<Vec<User>> {
        let req = self
            .request(Method::GET, "/rest/api/3/user/search")
            .query(&[("query", query)]);
        self.execute_json(req).await
    }

    // ------------------------------------------------------------------------
    // Issue APIs
    // ------------------------------------------------------------------------

    /// Searches for issues using Jira Query Language (JQL).
    ///
    /// # Arguments
    /// * `jql` - JQL search string (e.g. `project = DEV ORDER BY created DESC`).
    /// * `start_at` - Zero-based offset for pagination.
    /// * `max_results` - Maximum issues to return.
    pub async fn search_issues(
        &self,
        jql: &str,
        start_at: u32,
        max_results: u32,
    ) -> Result<SearchResults> {
        let payload = json!({
            "jql": jql,
            "startAt": start_at,
            "maxResults": max_results,
            "fields": [
                "summary",
                "status",
                "issuetype",
                "priority",
                "assignee",
                "reporter",
                "created",
                "updated",
                "labels"
            ]
        });

        let req = self
            .request(Method::POST, "/rest/api/3/search")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&payload);

        self.execute_json(req).await
    }

    /// Fetches detailed issue information by key or ID.
    pub async fn get_issue(&self, issue_key: &str) -> Result<Issue> {
        let endpoint = format!("/rest/api/3/issue/{}", issue_key.trim());
        let req = self.request(Method::GET, &endpoint).query(&[(
            "fields",
            "summary,status,issuetype,priority,assignee,reporter,created,updated,description,comment,labels",
        )]);
        self.execute_json(req).await
    }

    /// Creates a new Jira issue using REST API v3 with an optional ADF description.
    pub async fn create_issue(
        &self,
        project: &str,
        issue_type: &str,
        summary: &str,
        description: Option<&DocNode>,
        priority: Option<&str>,
        assignee_id: Option<&str>,
    ) -> Result<CreatedIssue> {
        let mut fields = json!({
            "project": { "key": project },
            "summary": summary,
            "issuetype": { "name": issue_type }
        });

        if let Some(desc) = description {
            fields["description"] = json!(desc);
        }

        if let Some(p) = priority {
            fields["priority"] = json!({ "name": p });
        }

        if let Some(aid) = assignee_id {
            fields["assignee"] = json!({ "accountId": aid });
        }

        let payload = json!({ "fields": fields });
        let req = self
            .request(Method::POST, "/rest/api/3/issue")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&payload);

        self.execute_json(req).await
    }

    /// Updates existing issue fields (summary, description, priority).
    pub async fn edit_issue(
        &self,
        issue_key: &str,
        summary: Option<&str>,
        description: Option<&DocNode>,
        priority: Option<&str>,
    ) -> Result<()> {
        let mut fields = serde_json::Map::new();

        if let Some(s) = summary {
            fields.insert("summary".to_string(), json!(s));
        }

        if let Some(desc) = description {
            fields.insert("description".to_string(), json!(desc));
        }

        if let Some(p) = priority {
            fields.insert("priority".to_string(), json!({ "name": p }));
        }

        if fields.is_empty() {
            return Ok(());
        }

        let payload = json!({ "fields": fields });
        let endpoint = format!("/rest/api/3/issue/{}", issue_key.trim());
        let req = self
            .request(Method::PUT, &endpoint)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&payload);

        self.execute(req).await?;
        Ok(())
    }

    /// Assigns an issue to a user by Atlassian account ID (or unassigns if `None`).
    pub async fn assign_issue(&self, issue_key: &str, account_id: Option<&str>) -> Result<()> {
        let endpoint = format!("/rest/api/3/issue/{}/assignee", issue_key.trim());
        let payload = json!({ "accountId": account_id });

        let req = self
            .request(Method::PUT, &endpoint)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&payload);

        self.execute(req).await?;
        Ok(())
    }

    /// Fetches all available workflow transitions for an issue.
    pub async fn get_transitions(&self, issue_key: &str) -> Result<Vec<Transition>> {
        let endpoint = format!("/rest/api/3/issue/{}/transitions", issue_key.trim());
        let req = self.request(Method::GET, &endpoint);
        let resp: TransitionsResponse = self.execute_json(req).await?;
        Ok(resp.transitions)
    }

    /// Executes a workflow transition on an issue.
    pub async fn do_transition(
        &self,
        issue_key: &str,
        transition_id: &str,
        comment: Option<&DocNode>,
    ) -> Result<()> {
        let mut payload = json!({
            "transition": { "id": transition_id }
        });

        if let Some(c) = comment {
            payload["update"] = json!({
                "comment": [
                    { "add": { "body": c } }
                ]
            });
        }

        let endpoint = format!("/rest/api/3/issue/{}/transitions", issue_key.trim());
        let req = self
            .request(Method::POST, &endpoint)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&payload);

        self.execute(req).await?;
        Ok(())
    }

    /// Fetches comments for an issue.
    pub async fn get_comments(&self, issue_key: &str) -> Result<CommentsResponse> {
        let endpoint = format!("/rest/api/3/issue/{}/comment", issue_key.trim());
        let req = self.request(Method::GET, &endpoint);
        self.execute_json(req).await
    }

    /// Adds a new comment to an issue in Atlassian Document Format.
    pub async fn add_comment(&self, issue_key: &str, body: &DocNode) -> Result<Comment> {
        let endpoint = format!("/rest/api/3/issue/{}/comment", issue_key.trim());
        let payload = json!({ "body": body });

        let req = self
            .request(Method::POST, &endpoint)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&payload);

        self.execute_json(req).await
    }

    /// Logs work on an issue.
    pub async fn add_worklog(
        &self,
        issue_key: &str,
        time_spent: &str,
        comment: Option<&DocNode>,
    ) -> Result<Worklog> {
        let endpoint = format!("/rest/api/3/issue/{}/worklog", issue_key.trim());
        let mut payload = json!({ "timeSpent": time_spent });

        if let Some(c) = comment {
            payload["comment"] = json!(c);
        }

        let req = self
            .request(Method::POST, &endpoint)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&payload);

        self.execute_json(req).await
    }

    // ------------------------------------------------------------------------
    // Project APIs
    // ------------------------------------------------------------------------

    /// Lists accessible Jira projects.
    pub async fn list_projects(&self) -> Result<Vec<Project>> {
        let req = self.request(Method::GET, "/rest/api/3/project");
        self.execute_json(req).await
    }

    /// Gets details of a specific project by key or ID.
    pub async fn get_project(&self, project_key: &str) -> Result<Project> {
        let endpoint = format!("/rest/api/3/project/{}", project_key.trim());
        let req = self
            .request(Method::GET, &endpoint)
            .query(&[("expand", "description,lead,issueTypes")]);
        self.execute_json(req).await
    }

    // ------------------------------------------------------------------------
    // Agile Boards & Sprints APIs
    // ------------------------------------------------------------------------

    /// Lists Agile boards.
    pub async fn list_boards(&self) -> Result<Vec<Board>> {
        let req = self.request(Method::GET, "/rest/agile/1.0/board");
        let resp: BoardsResponse = self.execute_json(req).await?;
        Ok(resp.values)
    }

    /// Lists sprints belonging to an Agile board.
    pub async fn list_sprints(&self, board_id: u64, state: Option<&str>) -> Result<Vec<Sprint>> {
        let endpoint = format!("/rest/agile/1.0/board/{}/sprint", board_id);
        let mut req = self.request(Method::GET, &endpoint);
        if let Some(s) = state {
            req = req.query(&[("state", s)]);
        }
        let resp: SprintsResponse = self.execute_json(req).await?;
        Ok(resp.values)
    }

    /// Lists issues associated with a sprint.
    pub async fn list_sprint_issues(&self, sprint_id: u64) -> Result<SearchResults> {
        let endpoint = format!("/rest/agile/1.0/sprint/{}/issue", sprint_id);
        let req = self.request(Method::GET, &endpoint);
        self.execute_json(req).await
    }
}
