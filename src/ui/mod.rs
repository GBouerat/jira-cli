//! User interface, table formatting, and terminal presentation components.
//!
//! Submodules provide support for dynamic terminal tables ([`table`]), single-issue
//! inspectors ([`detail`]), and global output modes ([`mod@format`]).

pub mod detail;
pub mod format;
pub mod table;

pub use detail::render_issue_detail;
pub use format::OutputMode;
pub use table::{
    format_file_size, render_attachments_table, render_boards_table, render_changelog_table,
    render_comments_table, render_components_table, render_issue_links_table, render_issues_table,
    render_projects_table, render_sprints_table, render_transitions_table, render_versions_table,
    render_watchers_table,
};
