//! Central application state, orchestration, and command-line input.
//!
//! # Modules
//!
//! - [`cli`] — Command-line parsing and browsing-root selection.
//! - [`controller`] — Application state and service coordination.
//! - [`domain`] — Workspace, explorer, preview, and recent-file state.

mod cli;
mod controller;
mod domain;

pub(crate) use cli::Cli;
pub(crate) use controller::{Controller, ExplorerActivation};
pub(crate) use domain::{
    DirectoryListing, ExplorerEntry, ExplorerEntryKind, ExplorerState, PreviewState,
    RECENT_FILE_LIMIT, RecentFilesState, Workspace,
};
