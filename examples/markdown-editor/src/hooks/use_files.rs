//! Shared file signals and their required context hook.

use std::path::PathBuf;

use leptatui::prelude::{RwSignal, expect_context};

/// File-related signals shared across pages and managed terminal sessions.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Files {
    /// Workspace-visible recent Markdown paths.
    pub(crate) recent_files: RwSignal<Vec<PathBuf>>,
    /// Complete persisted recent-file ordering, including other workspaces.
    pub(crate) stored_recent_files: RwSignal<Vec<PathBuf>>,
    /// Recoverable recent-file load or save error.
    pub(crate) recent_files_error: RwSignal<Option<String>>,
    /// Markdown path requested for editing after terminal restoration.
    pub(crate) edit_request: RwSignal<Option<PathBuf>>,
    /// Recoverable external-editor failure associated with one path.
    pub(crate) editor_failure: RwSignal<Option<EditorFailure>>,
}

impl Files {
    /// Creates shared file signals from loaded recent-file state.
    ///
    /// # Arguments
    ///
    /// * `recent_files` — Workspace-visible recent Markdown paths.
    /// * `stored_recent_files` — Complete persisted recent-file ordering.
    /// * `recent_files_error` — Recoverable recent-file load error.
    ///
    /// # Returns
    ///
    /// A [`Files`] value with empty editor handoff signals.
    pub(crate) fn new(
        recent_files: Vec<PathBuf>,
        stored_recent_files: Vec<PathBuf>,
        recent_files_error: Option<String>,
    ) -> Self {
        Self {
            recent_files: RwSignal::new(recent_files),
            stored_recent_files: RwSignal::new(stored_recent_files),
            recent_files_error: RwSignal::new(recent_files_error),
            edit_request: RwSignal::new(None),
            editor_failure: RwSignal::new(None),
        }
    }
}

/// External-editor failure associated with one requested path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EditorFailure {
    /// Markdown path supplied to the editor.
    pub(crate) path: PathBuf,
    /// Contextual editor launch or exit diagnostic.
    pub(crate) message: String,
}

/// Returns the shared file signals from the nearest context.
///
/// # Returns
///
/// A [`Files`] value cloned from the nearest matching context provider.
///
/// # Panics
///
/// Panics if no [`Files`] context is available.
pub(crate) fn use_files() -> Files {
    expect_context::<Files>()
}
