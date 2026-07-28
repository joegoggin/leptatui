//! Application controller for the Markdown editor.
//!
//! The controller validates initial state through infrastructure services and
//! provides page-facing access to explorer, viewer, and persistent recent-file
//! state.

use std::{
    io,
    path::{Path, PathBuf},
};

use crate::{
    core::{
        ExplorerEntryKind, ExplorerState, PreviewState, RECENT_FILE_LIMIT, RecentFilesState,
        Workspace,
    },
    services::{EditorProcess, FileSystem, RecentFilesStore},
};

/// Result of activating the selected explorer entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExplorerActivation {
    /// No explorer entry was selected.
    None,
    /// A directory was selected and the explorer handled the transition.
    Directory,
    /// A Markdown document was selected for the viewer.
    Document,
}

/// Application state and service boundaries used by the Markdown editor pages.
#[derive(Clone, Debug)]
pub(crate) struct Controller {
    /// Validated workspace displayed and navigated by the application.
    workspace: Workspace,
    /// Last valid explorer listing plus any recoverable navigation error.
    explorer: ExplorerState,
    /// Open Markdown path, view revision, and recoverable editor error.
    preview: PreviewState,
    /// Persisted recent Markdown paths and recoverable storage state.
    recent_files: RecentFilesState,
    /// Global persisted MRU paths, including entries for other workspaces.
    stored_recent_files: Vec<PathBuf>,
    /// Filesystem service used for anchored explorer transitions.
    filesystem: FileSystem,
    /// Process service used to launch the editor outside the managed terminal.
    editor_process: EditorProcess,
    /// Storage service used to retain recent paths between launches.
    recent_files_store: RecentFilesStore,
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
    /// * `editor_process` — Service used for external editor operations.
    ///
    /// # Returns
    ///
    /// A [`Controller`] containing validated startup state.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if the requested root cannot initialize a
    /// workspace.
    #[cfg(test)]
    pub(crate) fn initialize(
        requested_root: &Path,
        filesystem: FileSystem,
        editor_process: EditorProcess,
    ) -> io::Result<Self> {
        Self::initialize_with_store(
            requested_root,
            filesystem,
            editor_process,
            RecentFilesStore::memory(),
        )
    }

    /// Initializes the application with an explicit recent-file store.
    ///
    /// # Arguments
    ///
    /// * `requested_root` — User-selected or current-directory root.
    /// * `filesystem` — Service used to validate and canonicalize the root.
    /// * `editor_process` — Service used for external editor operations.
    /// * `recent_files_store` — Service used to load and save recent paths.
    ///
    /// # Returns
    ///
    /// A [`Controller`] containing validated startup and recent-file state.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if the requested root cannot initialize a
    /// workspace. Recent-file load failures become recoverable state.
    pub(crate) fn initialize_with_store(
        requested_root: &Path,
        filesystem: FileSystem,
        editor_process: EditorProcess,
        recent_files_store: RecentFilesStore,
    ) -> io::Result<Self> {
        let workspace = filesystem.validate_root(requested_root)?;
        let (stored_paths, recent_error) = match recent_files_store.load() {
            Ok(paths) => (paths, None),
            Err(error) => (Vec::new(), Some(error.to_string())),
        };
        let mut stored_recent_files = Vec::new();
        for path in stored_paths {
            if !stored_recent_files.contains(&path) {
                stored_recent_files.push(path);
            }
        }
        stored_recent_files.truncate(RECENT_FILE_LIMIT);

        let mut recent_paths = Vec::new();
        for path in &stored_recent_files {
            if let Ok(canonical) = filesystem.validate_markdown(&workspace, path)
                && !recent_paths.contains(&canonical)
            {
                recent_paths.push(canonical);
            }
        }
        let mut controller = Self {
            explorer: ExplorerState::new(workspace.root().to_path_buf()),
            preview: PreviewState::new(),
            recent_files: RecentFilesState::new(recent_paths, recent_error),
            stored_recent_files,
            workspace,
            filesystem,
            editor_process,
            recent_files_store,
        };
        controller.browse_root();

        Ok(controller)
    }

    /// Returns the validated workspace.
    ///
    /// # Returns
    ///
    /// A [`Workspace`] reference containing page-facing application state.
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
    /// A [`PreviewState`] reference containing the open document state.
    pub(crate) fn preview(&self) -> &PreviewState {
        &self.preview
    }

    /// Returns recent-file state.
    ///
    /// # Returns
    ///
    /// A [`RecentFilesState`] reference containing available paths and any
    /// recoverable persistence error.
    pub(crate) fn recent_files(&self) -> &RecentFilesState {
        &self.recent_files
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
    /// Directories replace the explorer listing. Markdown files select the
    /// path rendered by the existing path-backed Markdown view.
    ///
    /// # Returns
    ///
    /// An [`ExplorerActivation`] describing the selected entry kind.
    pub(crate) fn activate_selected(&mut self) -> ExplorerActivation {
        let Some(entry) = self.explorer.selected_entry().cloned() else {
            return ExplorerActivation::None;
        };

        match entry.kind() {
            ExplorerEntryKind::Directory => {
                self.browse(entry.path());
                ExplorerActivation::Directory
            }
            ExplorerEntryKind::Markdown => {
                self.open_preview(entry.path());
                ExplorerActivation::Document
            }
        }
    }

    /// Opens a recent Markdown path through the workspace boundary.
    ///
    /// Failed recent paths are removed from history while the attempted path
    /// remains available to the Viewer page for its file diagnostic.
    ///
    /// # Arguments
    ///
    /// * `path` — Persisted Markdown path selected on Home.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the document path validated successfully.
    pub(crate) fn open_recent(&mut self, path: &Path) -> bool {
        let loaded = self.open_preview(path);
        if !loaded {
            self.recent_files.remove(path);
            self.stored_recent_files.retain(|entry| entry != path);
            self.save_recent_files();
        }

        loaded
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

    /// Edits and invalidates the currently open Markdown preview.
    ///
    /// The caller invokes this method only after Leptatui has restored the
    /// terminal. A successful editor exit invalidates the path-backed Markdown
    /// view so it reloads the document from disk. Launch and exit failures
    /// become recoverable application errors while retaining the path for
    /// retry.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether a preview path was available to edit.
    pub(crate) fn edit_preview(&mut self) -> bool {
        let Some(path) = self.preview.path().map(Path::to_path_buf) else {
            return false;
        };

        match self.editor_process.edit(&path) {
            Ok(()) => {
                self.open_preview(&path);
            }
            Err(error) => self.preview.record_editor_error(error.to_string()),
        }

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

    /// Selects and validates a Markdown path for the Viewer page.
    ///
    /// The path-backed Markdown view owns source loading and file diagnostics.
    /// Successful validation promotes the canonical path in recent history;
    /// failed validation retains the requested path so the view can render its
    /// own failure page.
    ///
    /// # Arguments
    ///
    /// * `path` — Absolute selected Markdown path.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the path validated successfully.
    fn open_preview(&mut self, path: &Path) -> bool {
        match self.filesystem.validate_markdown(&self.workspace, path) {
            Ok(canonical_path) => {
                self.preview.open(canonical_path.clone());
                self.recent_files.promote(canonical_path.clone());
                self.stored_recent_files
                    .retain(|entry| entry != &canonical_path);
                self.stored_recent_files.insert(0, canonical_path);
                self.stored_recent_files.truncate(RECENT_FILE_LIMIT);
                self.save_recent_files();
                true
            }
            Err(_) => {
                self.preview.open(path.to_path_buf());
                false
            }
        }
    }

    /// Persists the current recent-file ordering.
    fn save_recent_files(&mut self) {
        let error = self
            .recent_files_store
            .save(&self.stored_recent_files)
            .err()
            .map(|error| error.to_string());
        self.recent_files.set_error(error);
    }
}
