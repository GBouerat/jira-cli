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
    render_boards_table, render_issues_table, render_projects_table, render_sprints_table,
    render_transitions_table,
};
