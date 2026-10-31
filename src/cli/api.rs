//! Direct raw Jira Cloud REST API execution (`jira api`).

use crate::client::JiraClient;
use crate::error::{JiraError, Result};
use clap::Args;
use reqwest::Method;

/// Arguments for `jira api <METHOD> <PATH> [-f BODY_FILE]`.
#[derive(Debug, Clone, Args)]
pub struct ApiArgs {
    /// HTTP method (GET, POST, PUT, DELETE, PATCH, etc.).
    pub method: String,

    /// API path relative to /rest/api/3 unless starting with /rest/ (e.g. 'issue/DEV-1' or '/rest/agile/1.0/board').
    pub path: String,

    /// Path to JSON file containing request body ('-' for stdin).
    #[arg(short = 'f', long = "body-file")]
    pub body_file: Option<String>,
}

/// Reads contents from a file path or standard input if `-`.
pub fn read_file_or_stdin(path_or_dash: &str) -> Result<String> {
    if path_or_dash == "-" {
        use std::io::Read;
        let mut buffer = String::new();
        std::io::stdin()
            .read_to_string(&mut buffer)
            .map_err(JiraError::Io)?;
        Ok(buffer)
    } else {
        std::fs::read_to_string(path_or_dash).map_err(|e| {
            JiraError::Generic(format!("Failed to read file '{}': {}", path_or_dash, e))
        })
    }
}

/// Dispatches raw API requests.
pub async fn handle_api_command(args: ApiArgs, client: &JiraClient) -> Result<()> {
    let method = Method::from_bytes(args.method.trim().to_uppercase().as_bytes())
        .map_err(|e| JiraError::Generic(format!("Invalid HTTP method '{}': {}", args.method, e)))?;

    let trimmed_path = args.path.trim();
    let normalized_path = if trimmed_path.starts_with("/rest/")
        || trimmed_path.starts_with("http://")
        || trimmed_path.starts_with("https://")
    {
        trimmed_path.to_string()
    } else if trimmed_path.starts_with('/') {
        format!("/rest/api/3{}", trimmed_path)
    } else {
        format!("/rest/api/3/{}", trimmed_path)
    };

    let body_json = if let Some(ref file_path) = args.body_file {
        let content = read_file_or_stdin(file_path)?;
        let parsed: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| JiraError::Generic(format!("Invalid JSON in request body: {}", e)))?;
        Some(parsed)
    } else {
        None
    };

    let resp = client
        .raw_api_call(method, &normalized_path, body_json.as_ref())
        .await?;

    let bytes = resp.bytes().await.map_err(JiraError::Network)?;
    if bytes.is_empty() {
        return Ok(());
    }

    if let Ok(json_val) = serde_json::from_slice::<serde_json::Value>(&bytes) {
        println!("{}", serde_json::to_string_pretty(&json_val)?);
    } else {
        println!("{}", String::from_utf8_lossy(&bytes));
    }

    Ok(())
}
