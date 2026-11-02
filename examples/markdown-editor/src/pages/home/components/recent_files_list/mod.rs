//! Recent-files list and its co-located stylesheet.
//!
//! # Modules
//!
//! - [`component`] — Recent-file list and empty state.
//! - [`style`] — Recent-files list stylesheet registration.

mod component;
mod style;

pub(in crate::pages::home) use component::{RecentFilesList, RecentFilesListProps};
