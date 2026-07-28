//! Components owned by the Home page.
//!
//! # Modules
//!
//! - [`recent_file_entry`] — One actionable recent-file row.
//! - [`recent_files_list`] — Recent-file list, empty state, and warning.

mod recent_file_entry;
mod recent_files_list;

pub(super) use recent_file_entry::{RecentFileEntry, RecentFileEntryProps};
pub(super) use recent_files_list::{RecentFilesList, RecentFilesListProps};
