//! Application controller for the Markdown editor.
//!
//! The controller validates initial state through infrastructure services and
//! provides UI-facing access to the resulting domain values.

use std::{io, path::Path};

use crate::{
    domain::{ExplorerEntryKind, ExplorerState, PreviewState, Workspace},
    editor_process::EditorProcess,
    filesystem::FileSystem,
};

/// Application state and service boundaries used by the Markdown editor UI.
#[derive(Clone, Debug)]
pub(crate) struct Controller {
    /// Validated workspace displayed and navigated by the application.
    workspace: Workspace,
    /// Last valid explorer listing plus any recoverable navigation error.
    explorer: ExplorerState,
    /// Open Markdown source or its recoverable read error.
    preview: PreviewState,
    /// Filesystem service used for anchored explorer transitions.
    filesystem: FileSystem,
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
        let mut controller = Self {
            explorer: ExplorerState::new(workspace.root().to_path_buf()),
            preview: PreviewState::new(),
            workspace,
            filesystem,
            _editor_process: editor_process,
        };
        controller.browse_root();

        Ok(controller)
    }

    /// Returns the validated workspace.
    ///
    /// # Returns
    ///
    /// A [`Workspace`] reference containing UI-facing application state.
    pub(crate) fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// Returns the current recoverable explorer state.
    ///
    /// # Returns
    ///
    /// An [`ExplorerState`] reference containing the current listing and any
    /// visible error.
    pub(crate) fn explorer(&self) -> &ExplorerState {
        &self.explorer
    }

    /// Returns the current Markdown preview.
    ///
    /// # Returns
    ///
    /// A [`PreviewState`] reference containing the open path and body state.
    pub(crate) fn preview(&self) -> &PreviewState {
        &self.preview
    }

    /// Moves the explorer selection toward the previous entry.
    pub(crate) fn select_previous(&mut self) {
        self.explorer.select_previous();
    }

    /// Moves the explorer selection toward the next entry.
    pub(crate) fn select_next(&mut self) {
        self.explorer.select_next();
    }

    /// Activates the selected directory or Markdown file.
    ///
    /// Directories replace the explorer listing. Markdown files replace the
    /// preview body. A failed file read remains visible in preview state.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether an explorer entry was selected.
    pub(crate) fn activate_selected(&mut self) -> bool {
        let Some(entry) = self.explorer.selected_entry().cloned() else {
            return false;
        };

        match entry.kind() {
            ExplorerEntryKind::Directory => {
                self.browse(entry.path());
            }
            ExplorerEntryKind::Markdown => {
                self.open_preview(entry.path());
            }
        }

        true
    }

    /// Reloads the currently open Markdown preview.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether a preview path was available to reload.
    pub(crate) fn reload_preview(&mut self) -> bool {
        let Some(path) = self.preview.path().map(Path::to_path_buf) else {
            return false;
        };

        self.open_preview(&path);
        true
    }

    /// Navigates to a requested directory within the workspace.
    ///
    /// A failed transition records its error while preserving the last valid
    /// directory and entries.
    ///
    /// # Arguments
    ///
    /// * `requested_directory` — Directory to resolve and list.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether navigation succeeded.
    pub(crate) fn browse(&mut self, requested_directory: &Path) -> bool {
        match self
            .filesystem
            .list_directory(&self.workspace, requested_directory)
        {
            Ok(listing) => {
                self.explorer.replace_listing(listing);
                true
            }
            Err(error) => {
                self.explorer.record_error(error.to_string());
                false
            }
        }
    }

    /// Navigates to the parent of the current explorer directory.
    ///
    /// Navigation at the configured root is a no-op. A parent read failure is
    /// recoverable and preserves the last valid listing.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the explorer moved to its parent.
    #[allow(
        dead_code,
        reason = "selection controls will call this anchored transition in the next phase"
    )]
    pub(crate) fn browse_parent(&mut self) -> bool {
        if self.explorer.directory() == self.workspace.root() {
            return false;
        }

        let Some(parent) = self.explorer.directory().parent().map(Path::to_path_buf) else {
            return false;
        };

        self.browse(&parent)
    }

    /// Loads the canonical workspace root into the explorer state.
    ///
    /// Root listing failures remain recoverable application state after the
    /// root itself has passed startup validation.
    fn browse_root(&mut self) {
        let root = self.workspace.root().to_path_buf();
        self.browse(&root);
    }

    /// Loads a Markdown path into recoverable preview state.
    ///
    /// # Arguments
    ///
    /// * `path` — Absolute selected Markdown path.
    fn open_preview(&mut self, path: &Path) {
        match self.filesystem.read_markdown(&self.workspace, path) {
            Ok(source) => self.preview.replace_document(path.to_path_buf(), source),
            Err(error) => self
                .preview
                .record_error(path.to_path_buf(), error.to_string()),
        }
    }
}
