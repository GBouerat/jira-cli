# jira-cli

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org)
[![Jira API](https://img.shields.io/badge/Jira%20API-v3%20Cloud-blue.svg)](https://developer.atlassian.com/cloud/jira/platform/rest/v3/intro/)

A modern, developer-friendly Jira Cloud CLI built in Rust using Jira REST API v3, featuring native Atlassian Document Format (ADF) handling, Unix pipeline composability, and interactive workflows.

---

## Features

- **Jira Cloud REST API v3 & Native ADF:** Automatic conversion between Markdown and Jira's JSON-based Atlassian Document Format (ADF AST) for descriptions and comments.
- **Fast & Responsive:** Instant startup with async networking via `reqwest` and `tokio`.
- **Flexible Output Modes:** Beautiful terminal tables (`comfy-table`) by default, with `--output json` and `--output markdown` for `jq`, scripts, and CI/CD pipelines.
- **Fail-Safe & Resilient:** Automatic detection and handling of HTTP 429 rate limits respecting Jira's `Retry-After` headers.
- **Multi-Profile Configuration:** Seamlessly switch between work and personal instances with profiles stored in `~/.config/jira-cli/config.json` (restricted to file mode `0600`) or environment variables.
- **Shell Completions:** Built-in completion generators for `zsh` and `bash`.

---

## Installation

### From Source via Cargo

```bash
cargo install --path .
```

---

## Quick Start & Setup

### 1. Interactive Setup Wizard

Run the configuration wizard to store your Jira Cloud credentials:

```bash
jira config init
```

You will be prompted for:
- **Jira Instance URL:** (e.g. `https://my-team.atlassian.net`)
- **Atlassian Account Email:** (e.g. `alex@example.com`)
- **Atlassian API Token:** (Generate at [Atlassian API Tokens](https://id.atlassian.com/manage-profile/security/api-tokens))
- **Default Project Key:** (e.g. `DEV`)

### 2. Verify Connectivity

Test your credentials and connection:

```bash
jira config test
```

### 3. Environment Variables (CI/CD or Headless)

You can also configure `jira-cli` via environment variables without creating a config file:

```bash
export JIRA_INSTANCE_URL="https://my-team.atlassian.net"
export JIRA_USER_EMAIL="alex@example.com"
export JIRA_API_TOKEN="ATATT3xFfGF0..."
export JIRA_DEFAULT_PROJECT="DEV"
```

Configuration resolution precedence follows: **CLI Flags > Environment Variables > Config File Profile**.

---

## Usage Guide

### Issue Management (`jira issue`)

#### List & Search Issues

```bash
# List recent issues in your default project
jira issue list

# Filter by project, status, and assignee
jira issue list -P DEV --status "In Progress" --assignee @me

# Custom Jira Query Language (JQL)
jira issue list --jql "project = DEV AND priority in (High, Blocker) ORDER BY created DESC"

# Output as JSON for piping into jq
jira issue list -o json | jq '.[0].fields.summary'

# Output as Markdown table
jira issue list -o markdown
```

#### View Issue Details

Displays metadata, rendered ADF description, and comments:

```bash
jira issue view DEV-101
```

#### Create an Issue

Create interactively, passing flags directly, or reading descriptions from files/stdin:

```bash
# Interactive prompts
jira issue create

# Direct flags with Markdown description
jira issue create \
  -P DEV \
  -t Bug \
  -s "Fix memory leak in background worker" \
  -d "### Root Cause\nThe worker does not flush the channel buffer on termination.\n\n- [ ] Add flush call\n- [ ] Verify test suite" \
  --priority High \
  --assignee @me

# Create subtask under a parent issue (automatically uses project's subtask type and parent's project)
jira issue create --parent DEV-100 -s "Implement backend migration"

# Description from a file or stdin (-f / --body-file)
jira issue create -P DEV -s "Implement OAuth2 flow" -f ./specs/oauth2.md

# Set labels and arbitrary / custom fields (JSON parsed if valid, else string)
jira issue create \
  -P DEV \
  -s "Add enterprise telemetry" \
  --label telemetry \
  --label enterprise \
  --field customfield_16375=N/A \
  --field 'components=[{"name":"Telemetry"}]'

# Preview request payload without creating (--dry-run)
jira issue create -P DEV -s "Test task" --dry-run
```

#### Edit an Issue

```bash
# Update summary, priority, or description
jira issue edit DEV-101 -s "New summary title" --priority Highest

# Description from a Markdown file or stdin
jira issue edit DEV-101 -f ./notes/update.md

# Replace all labels
jira issue edit DEV-101 --label frontend --label v2

# Add or remove specific labels using update verbs
jira issue edit DEV-101 --add-label approved --remove-label needs-review

# Update arbitrary / custom fields
jira issue edit DEV-101 --field customfield_10010=42 --field customfield_16375="N/A"

# Preview update payload without sending (--dry-run)
jira issue edit DEV-101 --add-label hotfix --dry-run
```

#### Batch Create Subtasks from a File

Batch-create subtasks under a parent issue from a JSON file. Entries whose summary already exists as a subtask on the parent are automatically skipped, allowing safe reruns after failures:

```bash
# subtasks.json:
# [
#   { "summary": "Design database schema", "description": "Markdown specs...", "priority": "High" },
#   { "summary": "Implement repository layer", "description": "CRUD operations" }
# ]

jira issue create-subtasks DEV-100 subtasks.json

# Limit maximum creations (e.g. create one to test rendering before proceeding)
jira issue create-subtasks DEV-100 subtasks.json --limit 1

# Apply common labels and custom fields to all created subtasks
jira issue create-subtasks DEV-100 subtasks.json \
  -l sprint-42 \
  --field customfield_16375=N/A

# Dry run to inspect payloads without creating
jira issue create-subtasks DEV-100 subtasks.json --dry-run
```

#### Multi-Issue Priority

Update the priority of multiple issues in a single invocation:

```bash
jira issue set-priority Blocker DEV-101 DEV-102 DEV-103
```

#### Assign an Issue

```bash
# Assign to yourself
jira issue assign DEV-101 @me

# Assign to a teammate
jira issue assign DEV-101 alex@example.com

# Unassign
jira issue assign DEV-101 --unassign
```

#### Workflow Transitions

```bash
# View available workflow transitions for an issue
jira issue transition DEV-101

# Advance to a status
jira issue transition DEV-101 "In Progress"

# Advance with a transition comment
jira issue transition DEV-101 Done -m "Resolved in release v1.4.0"
```

#### Comments & Worklogs

```bash
# List comments on an issue (displays comment IDs in terminal tables, text, and JSON)
jira issue comment DEV-101

# Add a comment with inline Markdown
jira issue comment DEV-101 "Investigating this issue now. See commit `a1b2c3d`."

# Add a comment from a Markdown file or stdin (-f / --body-file)
jira issue comment DEV-101 -f ./notes/analysis.md
cat ./notes/analysis.md | jira issue comment DEV-101 -f -

# Retrieve a single comment by ID
jira issue comment-get DEV-101 10050

# Output raw ADF body untouched (ideal for scripts to rewrite and send back)
jira issue comment-get DEV-101 10050 -o json > comment_adf.json

# Edit comment from Markdown (compiled to ADF)
jira issue comment-edit DEV-101 10050 -f ./notes/updated_comment.md

# Edit comment from raw ADF JSON (sent untouched without lossy markdown round-tripping)
jira issue comment-edit DEV-101 10050 --adf comment_adf.json

# Preview comment edit payload without sending (--dry-run)
jira issue comment-edit DEV-101 10050 -f ./notes/updated_comment.md --dry-run

# Delete a comment permanently (prompts for confirmation)
jira issue comment-delete DEV-101 10050

# Delete comment skipping confirmation prompt (-y)
jira issue comment-delete DEV-101 10050 -y

# Log time spent
jira issue worklog DEV-101 "2h 30m" -m "Implemented unit and integration tests"
```

#### Open in Browser

```bash
jira issue open DEV-101
```

#### Delete an Issue

```bash
# Delete an issue (prompts for confirmation)
jira issue delete DEV-101

# Force deletion without confirmation prompt
jira issue delete DEV-101 -y

# Delete issue along with all its subtasks
jira issue delete DEV-101 --delete-subtasks -y
```

#### Attachments

```bash
# Upload a file attachment to an issue
jira issue attach DEV-101 ./debug.log

# List attachments on an issue
jira issue attachments DEV-101
```

#### Issue Links

```bash
# Link two issues together with a relationship (default: "Relates")
jira issue link DEV-101 DEV-102

# Link with a custom relationship type and optional comment
jira issue link DEV-101 DEV-102 -t Blocks -m "DEV-101 blocks release of DEV-102"

# List issue links attached to an issue
jira issue links DEV-101

# Remove an issue link by link ID
jira issue unlink 10042
```

#### Watchers

```bash
# Add yourself as a watcher
jira issue watch DEV-101

# Add another user as a watcher
jira issue watch DEV-101 -u alex@example.com

# Remove yourself from watchers
jira issue unwatch DEV-101

# List watchers on an issue
jira issue watchers DEV-101
```

#### Issue History (Changelog)

```bash
# View field modification history (status changes, reassignments, etc.)
jira issue history DEV-101

# Limit number of history records
jira issue history DEV-101 --limit 20
```

---

### Sprint & Agile Board Management (`jira sprint`)

```bash
# List Agile Scrum/Kanban boards
jira sprint boards

# List sprints for a board
jira sprint list --board 12

# List only active sprints
jira sprint list --board 12 --state active

# List issues in a sprint
jira sprint issues 42
```

---

### Project Queries (`jira project`)

```bash
# List accessible projects
jira project list

# View project lead and available issue types
jira project view DEV

# List release versions for a project
jira project versions DEV

# Create a new release version
jira project create-version DEV --name "v2.0.0" --description "Major release" --release-date "2026-12-01"

# List project components
jira project components DEV
```

---

### Direct Raw REST API Access (`jira api`)

Perform arbitrary REST API calls against Jira Cloud while reusing the active profile's authentication, User-Agent, base URL resolution, and automatic HTTP 429 rate-limit backoff. PATH is relative to `/rest/api/3` unless it starts with `/rest/` or an absolute URL:

```bash
# Query any REST API v3 resource (relative path)
jira api GET issue/DEV-101

# Query Jira Agile API
jira api GET /rest/agile/1.0/board

# POST request with JSON body from a file
jira api POST issue/DEV-101/worklog -f ./payload.json

# POST request with JSON body from standard input
echo '{"timeSpent": "1h 30m"}' | jira api POST issue/DEV-101/worklog -f -

# Errors return non-zero exit codes with Jira's error payload printed to stderr
```

---

### Shell Completions

Generate shell completions for your active shell:

```bash
# For Zsh
jira completions zsh > ~/.zsh/completion/_jira

# For Bash
jira completions bash > ~/.local/share/bash-completion/completions/jira
```

---

## Architecture & Design

For technical architecture details, Atlassian Document Format (ADF) parsing design, and rate-limiting middleware, see [ARCHITECTURE.md](ARCHITECTURE.md).

---

## License

Dual-licensed under either:
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)

at your option.

---

*Generated entirely by Gemini through Antigravity.*
