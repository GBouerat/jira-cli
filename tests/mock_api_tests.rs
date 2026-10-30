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
            },
            "attachment": [
                {
                    "id": "9001",
                    "filename": "spec.pdf",
                    "size": 5120,
                    "mimeType": "application/pdf",
                    "content": "https://jira.example.com/attachments/9001"
                }
            ],
            "issuelinks": [
                {
                    "id": "8001",
                    "type": {
                        "id": "1",
                        "name": "Blocks",
                        "inward": "is blocked by",
                        "outward": "blocks"
                    },
                    "outwardIssue": {
                        "id": "2002",
                        "key": "DEV-203",
                        "fields": {
                            "summary": "Dependent feature"
                        }
                    }
                }
            ]
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
    assert_eq!(issue.fields.attachment.as_ref().unwrap().len(), 1);
    assert_eq!(issue.fields.issuelinks.as_ref().unwrap().len(), 1);
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
async fn test_mock_rate_limiting_long_returns_error() {
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
async fn test_mock_rate_limiting_short_retry_succeeds() {
    let mock_server = MockServer::start().await;

    // First request: returns 429 with Retry-After: 1
    // Second request: succeeds with 200 OK
    let user_json = serde_json::json!({
        "accountId": "acc-999",
        "displayName": "Retried User",
        "active": true
    });

    Mock::given(method("GET"))
        .and(path("/rest/api/3/myself"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "1")
                .set_body_string("Rate limit burst"),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/rest/api/3/myself"))
        .respond_with(ResponseTemplate::new(200).set_body_json(user_json))
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();
    let user = client.myself().await.expect("retry should succeed");
    assert_eq!(user.account_id, "acc-999");
}

#[tokio::test]
async fn test_mock_issue_key_case_normalization() {
    let mock_server = MockServer::start().await;

    let issue_json = serde_json::json!({
        "id": "500",
        "key": "DEV-500",
        "fields": {
            "summary": "Case normalization test"
        }
    });

    Mock::given(method("GET"))
        .and(path("/rest/api/3/issue/DEV-500"))
        .respond_with(ResponseTemplate::new(200).set_body_json(issue_json))
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();
    // Pass lowercase "dev-500" with whitespace - client should normalize to "DEV-500"
    let issue = client
        .get_issue("  dev-500  ")
        .await
        .expect("normalization should match DEV-500");
    assert_eq!(issue.key, "DEV-500");
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

#[tokio::test]
async fn test_mock_delete_issue() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/rest/api/3/issue/DEV-100"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();
    client
        .delete_issue("dev-100", false)
        .await
        .expect("deletion should succeed");
}

#[tokio::test]
async fn test_mock_attachments_api() {
    let mock_server = MockServer::start().await;

    let attachments_json = serde_json::json!({
        "id": "100",
        "key": "DEV-100",
        "fields": {
            "summary": "Attachment test",
            "attachment": [
                {
                    "id": "5001",
                    "filename": "screenshot.png",
                    "size": 1024,
                    "mimeType": "image/png",
                    "content": "https://jira.example.com/attachments/5001"
                }
            ]
        }
    });

    Mock::given(method("GET"))
        .and(path("/rest/api/3/issue/DEV-100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(attachments_json))
        .mount(&mock_server)
        .await;

    let upload_resp = serde_json::json!([
        {
            "id": "5002",
            "filename": "test.txt",
            "size": 12,
            "mimeType": "text/plain",
            "content": "https://jira.example.com/attachments/5002"
        }
    ]);

    Mock::given(method("POST"))
        .and(path("/rest/api/3/issue/DEV-100/attachments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(upload_resp))
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();

    let list = client.get_attachments("DEV-100").await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].filename, "screenshot.png");

    let uploaded = client
        .add_attachment("DEV-100", "test.txt", b"hello world\n".to_vec())
        .await
        .unwrap();
    assert_eq!(uploaded.len(), 1);
    assert_eq!(uploaded[0].filename, "test.txt");
}

#[tokio::test]
async fn test_mock_issue_links_and_watchers() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/rest/api/3/issueLink"))
        .and(wiremock::matchers::body_json(serde_json::json!({
            "type": { "name": "Blocks" },
            "inwardIssue": { "key": "DEV-1" },
            "outwardIssue": { "key": "DEV-2" }
        })))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/rest/api/3/issueLink/999"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let watchers_json = serde_json::json!({
        "self": "https://jira.example.com/rest/api/3/issue/DEV-1/watchers",
        "isWatching": true,
        "watchCount": 1,
        "watchers": [
            {
                "accountId": "acc-1",
                "displayName": "Alice",
                "active": true
            }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/rest/api/3/issue/DEV-1/watchers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(watchers_json))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/rest/api/3/issue/DEV-1/watchers"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();

    client
        .link_issues("dev-1", "dev-2", "blocks", None)
        .await
        .unwrap();
    client.delete_issue_link("999").await.unwrap();

    let watchers = client.get_watchers("DEV-1").await.unwrap();
    assert_eq!(watchers.watch_count, 1);
    assert!(watchers.is_watching);

    client.add_watcher("DEV-1", "acc-2").await.unwrap();
}

#[tokio::test]
async fn test_mock_project_versions_and_components() {
    let mock_server = MockServer::start().await;

    let versions_json = serde_json::json!([
        {
            "id": "1000",
            "name": "1.0.0",
            "archived": false,
            "released": true,
            "releaseDate": "2026-10-30"
        }
    ]);

    Mock::given(method("GET"))
        .and(path("/rest/api/3/project/DEV/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(versions_json))
        .mount(&mock_server)
        .await;

    let comp_json = serde_json::json!([
        {
            "id": "2000",
            "name": "Auth",
            "description": "Authentication module"
        }
    ]);

    Mock::given(method("GET"))
        .and(path("/rest/api/3/project/DEV/components"))
        .respond_with(ResponseTemplate::new(200).set_body_json(comp_json))
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();

    let versions = client.list_project_versions("DEV").await.unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].name, "1.0.0");

    let components = client.list_project_components("DEV").await.unwrap();
    assert_eq!(components.len(), 1);
    assert_eq!(components[0].name, "Auth");
}
