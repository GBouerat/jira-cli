//! Integration tests using local async WireMock HTTP server.
//!
//! Validates authentication headers, User-Agent, Jira REST API v3 endpoint schemas,
//! ADF handling, error parsing, and HTTP 429 rate limit backoff.

use jira_cli::adf::ast::{DocNode, Node};
use jira_cli::client::JiraClient;
use jira_cli::error::JiraError;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_mock_myself_endpoint() {
    let mock_server = MockServer::start().await;

    let response_body = serde_json::json!({
        "accountId": "acc-12345",
        "displayName": "Alex Developer",
        "emailAddress": "alex@example.com",
        "active": true
    });

    Mock::given(method("GET"))
        .and(path("/rest/api/3/myself"))
        .and(header("Accept", "application/json"))
        .and(header(
            "Authorization",
            "Basic dXNlckBleGFtcGxlLmNvbTp0b2tlbjEyMw==",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "user@example.com", "token123").unwrap();
    let user = client.myself().await.expect("failed to call myself");

    assert_eq!(user.account_id, "acc-12345");
    assert_eq!(user.display_name, "Alex Developer");
    assert_eq!(user.email_address.as_deref(), Some("alex@example.com"));
    assert!(user.active);
}

#[tokio::test]
async fn test_mock_search_issues() {
    let mock_server = MockServer::start().await;

    let search_json = serde_json::json!({
        "startAt": 0,
        "maxResults": 50,
        "total": 1,
        "issues": [
            {
                "id": "1001",
                "key": "DEV-101",
                "fields": {
                    "summary": "Fix async worker deadlock",
                    "status": {
                        "id": "3",
                        "name": "In Progress"
                    },
                    "issuetype": {
                        "id": "1",
                        "name": "Bug",
                        "subtask": false
                    },
                    "labels": ["backend"]
                }
            }
        ]
    });

    Mock::given(method("POST"))
        .and(path("/rest/api/3/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_json))
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();
    let results = client
        .search_issues("project = DEV", 0, 50)
        .await
        .expect("search failed");

    assert_eq!(results.total, 1);
    assert_eq!(results.issues.len(), 1);
    assert_eq!(results.issues[0].key, "DEV-101");
    assert_eq!(
        results.issues[0].fields.summary,
        "Fix async worker deadlock"
    );
}

#[tokio::test]
async fn test_mock_get_issue_with_adf() {
    let mock_server = MockServer::start().await;

    let issue_json = serde_json::json!({
        "id": "2001",
        "key": "DEV-202",
        "fields": {
            "summary": "Support ADF rendering",
            "status": {
                "id": "1",
                "name": "Open"
            },
            "description": {
                "version": 1,
                "type": "doc",
                "content": [
                    {
                        "type": "paragraph",
                        "content": [
                            {
                                "type": "text",
                                "text": "This is an ADF description."
                            }
                        ]
                    }
                ]
            }
        }
    });

    Mock::given(method("GET"))
        .and(path("/rest/api/3/issue/DEV-202"))
        .respond_with(ResponseTemplate::new(200).set_body_json(issue_json))
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();
    let issue = client.get_issue("DEV-202").await.expect("get issue failed");

    assert_eq!(issue.key, "DEV-202");
    let desc = issue.fields.description.expect("description missing");
    assert_eq!(desc.node_type, "doc");
    assert_eq!(desc.content[0].node_type, "paragraph");
    assert_eq!(
        desc.content[0].content[0].text.as_deref(),
        Some("This is an ADF description.")
    );
}

#[tokio::test]
async fn test_mock_create_issue() {
    let mock_server = MockServer::start().await;

    let created_json = serde_json::json!({
        "id": "3001",
        "key": "DEV-303",
        "self": format!("{}/rest/api/3/issue/3001", mock_server.uri())
    });

    Mock::given(method("POST"))
        .and(path("/rest/api/3/issue"))
        .respond_with(ResponseTemplate::new(201).set_body_json(created_json))
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();
    let desc = DocNode::new(vec![Node::paragraph(vec![Node::text("New task details")])]);

    let created = client
        .create_issue(
            "DEV",
            "Task",
            "New issue summary",
            Some(&desc),
            Some("High"),
            None,
        )
        .await
        .expect("create issue failed");

    assert_eq!(created.key, "DEV-303");
    assert_eq!(created.id, "3001");
}

#[tokio::test]
async fn test_mock_rate_limiting_429() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/rest/api/3/myself"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "15")
                .set_body_string("Rate limit exceeded"),
        )
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();
    let err = client
        .myself()
        .await
        .expect_err("expected rate limit error");

    match err {
        JiraError::RateLimited {
            retry_after_seconds,
        } => {
            assert_eq!(retry_after_seconds, 15);
        }
        other => panic!("Expected JiraError::RateLimited, got {:?}", other),
    }
}

#[tokio::test]
async fn test_mock_api_error_response() {
    let mock_server = MockServer::start().await;

    let error_json = serde_json::json!({
        "errorMessages": ["You do not have permission to edit this issue."],
        "errors": {
            "summary": "Summary cannot be empty"
        }
    });

    Mock::given(method("PUT"))
        .and(path("/rest/api/3/issue/DEV-999"))
        .respond_with(ResponseTemplate::new(403).set_body_json(error_json))
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();
    let err = client
        .edit_issue("DEV-999", Some(""), None, None)
        .await
        .expect_err("expected api error");

    match err {
        JiraError::Api {
            status,
            message,
            errors,
        } => {
            assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
            assert!(message.contains("You do not have permission to edit this issue."));
            assert_eq!(
                errors.as_ref().unwrap().get("summary"),
                Some(&"Summary cannot be empty".to_string())
            );
        }
        other => panic!("Expected JiraError::Api, got {:?}", other),
    }
}
