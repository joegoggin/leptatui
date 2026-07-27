//! Application controller for the Markdown editor.
//!
//! The controller validates initial state through infrastructure services and
//! provides UI-facing access to the resulting domain values.

use std::{io, path::Path};

use crate::{domain::Workspace, editor_process::EditorProcess, filesystem::FileSystem};

/// Application state and service boundaries used by the Markdown editor UI.
#[derive(Clone, Debug)]
pub(crate) struct Controller {
    /// Validated workspace displayed and navigated by the application.
    workspace: Workspace,
    /// Filesystem service retained for future explorer transitions.
    _filesystem: FileSystem,
    /// Process service retained for future external-editor transitions.
    _editor_process: EditorProcess,
}

impl Controller {
    /// Initializes the application from a requested browsing root.
    ///
    /// Root validation completes here, before the caller constructs a
    /// Leptatui [`App`](leptatui::App) or enters a managed terminal session.
    ///
    /// # Arguments
    ///
    /// * `requested_root` — User-selected or current-directory root.
    /// * `filesystem` — Service used to validate and canonicalize the root.
    /// * `editor_process` — Service reserved for external editor operations.
    ///
    /// # Returns
    ///
    /// A [`Controller`] containing validated startup state.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if the requested root cannot initialize a
    /// workspace.
    pub(crate) fn initialize(
        requested_root: &Path,
        filesystem: FileSystem,
        editor_process: EditorProcess,
    ) -> io::Result<Self> {
        let workspace = filesystem.validate_root(requested_root)?;

        Ok(Self {
            workspace,
            _filesystem: filesystem,
            _editor_process: editor_process,
        })
    }

    /// Returns the validated workspace.
    ///
    /// # Returns
    ///
    /// A [`Workspace`] reference containing UI-facing application state.
    pub(crate) fn workspace(&self) -> &Workspace {
        &self.workspace
    }
}
