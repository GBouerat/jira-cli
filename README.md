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

Create interactively or passing all arguments directly:

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
```

#### Edit an Issue

```bash
jira issue edit DEV-101 -s "New summary title" --priority Highest
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
# List comments on an issue
jira issue comment DEV-101

# Add a comment (supports Markdown)
jira issue comment DEV-101 "Investigating this issue now. See commit `a1b2c3d`."

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
