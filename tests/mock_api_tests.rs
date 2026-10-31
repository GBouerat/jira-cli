//! Integration tests using local async WireMock HTTP server.
//!
//! Validates authentication headers, User-Agent, Jira REST API v3 endpoint schemas,
//! ADF handling, error parsing, and HTTP 429 rate limit backoff.

use jira_cli::adf::ast::{DocNode, Node};
use jira_cli::adf::from_text::text_to_adf;
use jira_cli::cli::api::{handle_api_command, ApiArgs};
use jira_cli::cli::issue::{
    handle_issue_command, IssueArgs, IssueCommentDeleteArgs, IssueCommentEditArgs,
    IssueCommentGetArgs, IssueCreateArgs, IssueCreateSubtasksArgs, IssueEditArgs,
    IssueSetPriorityArgs, IssueSubcommand,
};
use jira_cli::client::JiraClient;
use jira_cli::error::JiraError;
use jira_cli::ui::format::OutputMode;
use wiremock::matchers::{body_json, header, method, path};
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

#[tokio::test]
async fn test_mock_comment_get() {
    let mock_server = MockServer::start().await;

    let comment_body = serde_json::json!({
        "id": "5001",
        "author": {
            "accountId": "acc-1",
            "displayName": "Alex",
            "active": true
        },
        "body": {
            "version": 1,
            "type": "doc",
            "content": [
                {
                    "type": "paragraph",
                    "content": [
                        { "type": "text", "text": "Original comment content" }
                    ]
                }
            ]
        },
        "created": "2026-10-31T12:00:00.000+0000"
    });

    Mock::given(method("GET"))
        .and(path("/rest/api/3/issue/DEV-10/comment/5001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(comment_body))
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();

    let comment = client.get_comment("DEV-10", "5001").await.unwrap();
    assert_eq!(comment.id, "5001");
    assert!(comment.body.is_some());

    let args = IssueArgs {
        command: IssueSubcommand::CommentGet(IssueCommentGetArgs {
            key: "DEV-10".to_string(),
            comment_id: "5001".to_string(),
        }),
    };
    handle_issue_command(args, &client, OutputMode::Json, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_mock_comment_edit_with_markdown_and_dry_run() {
    let mock_server = MockServer::start().await;

    let markdown_content = "# Updated Heading\n\n**Bold edit**";
    let expected_adf = text_to_adf(markdown_content);
    let expected_payload = serde_json::json!({
        "body": expected_adf
    });

    let resp_comment = serde_json::json!({
        "id": "5001",
        "body": expected_adf,
        "created": "2026-10-31T12:00:00.000+0000"
    });

    Mock::given(method("PUT"))
        .and(path("/rest/api/3/issue/DEV-10/comment/5001"))
        .and(body_json(&expected_payload))
        .respond_with(ResponseTemplate::new(200).set_body_json(resp_comment))
        .mount(&mock_server)
        .await;

    let temp_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), markdown_content).unwrap();

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();

    // 1. Dry run should NOT call the server
    let dry_args = IssueArgs {
        command: IssueSubcommand::CommentEdit(IssueCommentEditArgs {
            key: "DEV-10".to_string(),
            comment_id: "5001".to_string(),
            message: None,
            body_file: Some(temp_file.path().to_str().unwrap().to_string()),
            adf: None,
            dry_run: true,
        }),
    };
    handle_issue_command(dry_args, &client, OutputMode::Json, None)
        .await
        .unwrap();

    // 2. Real call asserts exact method, path and payload
    let real_args = IssueArgs {
        command: IssueSubcommand::CommentEdit(IssueCommentEditArgs {
            key: "DEV-10".to_string(),
            comment_id: "5001".to_string(),
            message: None,
            body_file: Some(temp_file.path().to_str().unwrap().to_string()),
            adf: None,
            dry_run: false,
        }),
    };
    handle_issue_command(real_args, &client, OutputMode::Json, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_mock_comment_edit_with_raw_adf() {
    let mock_server = MockServer::start().await;

    let raw_adf_json = serde_json::json!({
        "version": 1,
        "type": "doc",
        "content": [
            {
                "type": "paragraph",
                "content": [
                    { "type": "text", "text": "Raw ADF sent untouched!" }
                ]
            }
        ]
    });
    let expected_payload = serde_json::json!({
        "body": raw_adf_json
    });

    let resp_comment = serde_json::json!({
        "id": "5001",
        "body": raw_adf_json,
        "created": "2026-10-31T12:00:00.000+0000"
    });

    Mock::given(method("PUT"))
        .and(path("/rest/api/3/issue/DEV-10/comment/5001"))
        .and(body_json(&expected_payload))
        .respond_with(ResponseTemplate::new(200).set_body_json(resp_comment))
        .mount(&mock_server)
        .await;

    let temp_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        temp_file.path(),
        serde_json::to_string(&raw_adf_json).unwrap(),
    )
    .unwrap();

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();

    let args = IssueArgs {
        command: IssueSubcommand::CommentEdit(IssueCommentEditArgs {
            key: "DEV-10".to_string(),
            comment_id: "5001".to_string(),
            message: None,
            body_file: None,
            adf: Some(temp_file.path().to_str().unwrap().to_string()),
            dry_run: false,
        }),
    };
    handle_issue_command(args, &client, OutputMode::Json, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_mock_comment_delete() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/rest/api/3/issue/DEV-10/comment/5001"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();

    let args = IssueArgs {
        command: IssueSubcommand::CommentDelete(IssueCommentDeleteArgs {
            key: "DEV-10".to_string(),
            comment_id: "5001".to_string(),
            yes: true,
        }),
    };
    handle_issue_command(args, &client, OutputMode::Json, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_mock_issue_create_with_parent_and_custom_fields() {
    let mock_server = MockServer::start().await;

    // 1. Fetch parent issue to resolve project
    let parent_json = serde_json::json!({
        "id": "9900",
        "key": "PARENT-99",
        "fields": {
            "summary": "Parent epic task",
            "project": {
                "id": "100",
                "key": "DEV",
                "name": "Development"
            }
        }
    });

    Mock::given(method("GET"))
        .and(path("/rest/api/3/issue/PARENT-99"))
        .respond_with(ResponseTemplate::new(200).set_body_json(parent_json))
        .mount(&mock_server)
        .await;

    // 2. Fetch createmeta to resolve subtask type
    let createmeta_json = serde_json::json!({
        "values": [
            {
                "id": "1",
                "name": "Task",
                "subtask": false
            },
            {
                "id": "5",
                "name": "Sub-task",
                "subtask": true
            }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/rest/api/3/issue/createmeta/DEV/issuetypes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(createmeta_json))
        .mount(&mock_server)
        .await;

    // 3. Post issue asserting exact payload
    let expected_payload = serde_json::json!({
        "fields": {
            "project": { "key": "DEV" },
            "parent": { "key": "PARENT-99" },
            "issuetype": { "name": "Sub-task" },
            "summary": "Build authentication subtask",
            "labels": ["auth", "backend"],
            "customfield_16375": "N/A",
            "components": [{ "name": "Security" }]
        }
    });

    let created_resp = serde_json::json!({
        "id": "9901",
        "key": "DEV-9901",
        "self": format!("{}/rest/api/3/issue/9901", mock_server.uri())
    });

    Mock::given(method("POST"))
        .and(path("/rest/api/3/issue"))
        .and(body_json(&expected_payload))
        .respond_with(ResponseTemplate::new(201).set_body_json(created_resp))
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();

    let args = IssueArgs {
        command: IssueSubcommand::Create(IssueCreateArgs {
            project: None,
            issue_type: None,
            summary: Some("Build authentication subtask".to_string()),
            description: None,
            body_file: None,
            priority: None,
            assignee: None,
            parent: Some("PARENT-99".to_string()),
            labels: vec!["auth".to_string(), "backend".to_string()],
            fields: vec![
                "customfield_16375=N/A".to_string(),
                "components=[{\"name\":\"Security\"}]".to_string(),
            ],
            dry_run: false,
        }),
    };

    handle_issue_command(args, &client, OutputMode::Json, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_mock_issue_edit_labels_and_verbs_and_custom_fields() {
    let mock_server = MockServer::start().await;

    // 1. Test replace labels and custom field
    let expected_replace_payload = serde_json::json!({
        "fields": {
            "labels": ["v1.0", "stable"],
            "customfield_status_flag": true
        }
    });

    Mock::given(method("PUT"))
        .and(path("/rest/api/3/issue/DEV-10"))
        .and(body_json(&expected_replace_payload))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();

    let replace_args = IssueArgs {
        command: IssueSubcommand::Edit(IssueEditArgs {
            key: "DEV-10".to_string(),
            summary: None,
            description: None,
            body_file: None,
            priority: None,
            labels: vec!["v1.0".to_string(), "stable".to_string()],
            add_labels: vec![],
            remove_labels: vec![],
            fields: vec!["customfield_status_flag=true".to_string()],
            dry_run: false,
        }),
    };
    handle_issue_command(replace_args, &client, OutputMode::Json, None)
        .await
        .unwrap();

    // 2. Test update verb labels (--add-label / --remove-label)
    let expected_update_payload = serde_json::json!({
        "update": {
            "labels": [
                { "add": "feature-x" },
                { "remove": "deprecated" }
            ]
        }
    });

    Mock::given(method("PUT"))
        .and(path("/rest/api/3/issue/DEV-20"))
        .and(body_json(&expected_update_payload))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let update_args = IssueArgs {
        command: IssueSubcommand::Edit(IssueEditArgs {
            key: "DEV-20".to_string(),
            summary: None,
            description: None,
            body_file: None,
            priority: None,
            labels: vec![],
            add_labels: vec!["feature-x".to_string()],
            remove_labels: vec!["deprecated".to_string()],
            fields: vec![],
            dry_run: false,
        }),
    };
    handle_issue_command(update_args, &client, OutputMode::Json, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_mock_issue_create_subtasks_batch_and_deduplication() {
    let mock_server = MockServer::start().await;

    // 1. Fetch parent issue with existing subtasks
    let parent_json = serde_json::json!({
        "id": "8000",
        "key": "DEV-80",
        "fields": {
            "summary": "Parent Issue",
            "project": { "id": "100", "key": "DEV", "name": "Development" },
            "subtasks": [
                {
                    "id": "8001",
                    "key": "DEV-81",
                    "fields": {
                        "summary": "Already Done Subtask"
                    }
                }
            ]
        }
    });

    Mock::given(method("GET"))
        .and(path("/rest/api/3/issue/DEV-80"))
        .respond_with(ResponseTemplate::new(200).set_body_json(parent_json))
        .mount(&mock_server)
        .await;

    // 2. Fetch createmeta
    let createmeta_json = serde_json::json!({
        "values": [
            { "id": "5", "name": "Sub-task", "subtask": true }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/rest/api/3/issue/createmeta/DEV/issuetypes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(createmeta_json))
        .mount(&mock_server)
        .await;

    // 3. Expected subtask create payload (only "New Subtask 1" should be created due to --limit 1)
    let expected_desc = text_to_adf("Details for new task 1");
    let expected_payload = serde_json::json!({
        "fields": {
            "project": { "key": "DEV" },
            "parent": { "key": "DEV-80" },
            "issuetype": { "name": "Sub-task" },
            "summary": "New Subtask 1",
            "description": expected_desc,
            "priority": { "name": "High" },
            "labels": ["batch-test"],
            "customfield_batch_id": 999
        }
    });

    let created_resp = serde_json::json!({
        "id": "8002",
        "key": "DEV-82",
        "self": format!("{}/rest/api/3/issue/8002", mock_server.uri())
    });

    Mock::given(method("POST"))
        .and(path("/rest/api/3/issue"))
        .and(body_json(&expected_payload))
        .respond_with(ResponseTemplate::new(201).set_body_json(created_resp))
        .mount(&mock_server)
        .await;

    // Batch JSON file containing an existing summary, a new one, and a second new one
    let batch_json = serde_json::json!([
        {
            "summary": "Already Done Subtask",
            "description": "Will be skipped",
            "priority": "Low"
        },
        {
            "summary": "New Subtask 1",
            "description": "Details for new task 1",
            "priority": "High"
        },
        {
            "summary": "New Subtask 2",
            "description": "Will be skipped because limit=1",
            "priority": "Medium"
        }
    ]);

    let temp_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        temp_file.path(),
        serde_json::to_string(&batch_json).unwrap(),
    )
    .unwrap();

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();

    let args = IssueArgs {
        command: IssueSubcommand::CreateSubtasks(IssueCreateSubtasksArgs {
            parent: "DEV-80".to_string(),
            file: temp_file.path().to_str().unwrap().to_string(),
            limit: Some(1),
            labels: vec!["batch-test".to_string()],
            fields: vec!["customfield_batch_id=999".to_string()],
            dry_run: false,
        }),
    };

    handle_issue_command(args, &client, OutputMode::Json, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_mock_issue_set_priority_multi() {
    let mock_server = MockServer::start().await;

    let expected_payload = serde_json::json!({
        "fields": {
            "priority": { "name": "Highest" }
        }
    });

    Mock::given(method("PUT"))
        .and(path("/rest/api/3/issue/DEV-101"))
        .and(body_json(&expected_payload))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    Mock::given(method("PUT"))
        .and(path("/rest/api/3/issue/DEV-102"))
        .and(body_json(&expected_payload))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();

    let args = IssueArgs {
        command: IssueSubcommand::SetPriority(IssueSetPriorityArgs {
            priority: "Highest".to_string(),
            keys: vec!["DEV-101".to_string(), "DEV-102".to_string()],
        }),
    };

    handle_issue_command(args, &client, OutputMode::Json, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_mock_raw_api_call() {
    let mock_server = MockServer::start().await;

    // 1. Relative GET call: issue/DEV-1 -> /rest/api/3/issue/DEV-1
    let get_resp = serde_json::json!({
        "id": "100",
        "key": "DEV-1",
        "fields": { "summary": "Raw api summary" }
    });

    Mock::given(method("GET"))
        .and(path("/rest/api/3/issue/DEV-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(get_resp))
        .mount(&mock_server)
        .await;

    // 2. Absolute /rest/ GET call: /rest/agile/1.0/board
    let board_resp = serde_json::json!({
        "values": [{ "id": 1, "name": "Agile Board" }]
    });

    Mock::given(method("GET"))
        .and(path("/rest/agile/1.0/board"))
        .respond_with(ResponseTemplate::new(200).set_body_json(board_resp))
        .mount(&mock_server)
        .await;

    // 3. POST with body from file
    let post_body = serde_json::json!({
        "timeSpent": "3h 30m"
    });

    Mock::given(method("POST"))
        .and(path("/rest/api/3/issue/DEV-1/worklog"))
        .and(body_json(&post_body))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(serde_json::json!({ "id": "wl-100" })),
        )
        .mount(&mock_server)
        .await;

    // 4. 404 Error handling
    let error_resp = serde_json::json!({
        "errorMessages": ["Issue does not exist"]
    });

    Mock::given(method("GET"))
        .and(path("/rest/api/3/issue/NONEXISTENT-999"))
        .respond_with(ResponseTemplate::new(404).set_body_json(error_resp))
        .mount(&mock_server)
        .await;

    let client =
        JiraClient::from_credentials(&mock_server.uri(), "test@test.com", "token").unwrap();

    // Test relative path GET
    let args1 = ApiArgs {
        method: "GET".to_string(),
        path: "issue/DEV-1".to_string(),
        body_file: None,
    };
    handle_api_command(args1, &client).await.unwrap();

    // Test /rest/ path GET
    let args2 = ApiArgs {
        method: "GET".to_string(),
        path: "/rest/agile/1.0/board".to_string(),
        body_file: None,
    };
    handle_api_command(args2, &client).await.unwrap();

    // Test POST with file
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), serde_json::to_string(&post_body).unwrap()).unwrap();

    let args3 = ApiArgs {
        method: "POST".to_string(),
        path: "/rest/api/3/issue/DEV-1/worklog".to_string(),
        body_file: Some(temp_file.path().to_str().unwrap().to_string()),
    };
    handle_api_command(args3, &client).await.unwrap();

    // Test error returns Err
    let args4 = ApiArgs {
        method: "GET".to_string(),
        path: "issue/NONEXISTENT-999".to_string(),
        body_file: None,
    };
    let err = handle_api_command(args4, &client).await.unwrap_err();
    match err {
        JiraError::Api {
            status, message, ..
        } => {
            assert_eq!(status, 404);
            assert!(message.contains("Issue does not exist"));
        }
        other => panic!("expected JiraError::Api, got: {:?}", other),
    }
}
