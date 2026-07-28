//! Workspace resources and their required context hook.

#[cfg(test)]
use std::path::Path;

use leptatui::prelude::expect_context;

use crate::services::{FileSystem, Workspace};

/// Validated workspace and its bounded filesystem service.
#[derive(Clone, Debug)]
pub(crate) struct WorkspaceContext {
    /// Validated workspace that bounds browsing and Markdown access.
    pub(crate) workspace: Workspace,
    /// Filesystem service used for workspace-scoped operations.
    pub(crate) filesystem: FileSystem,
}

impl WorkspaceContext {
    /// Creates a workspace context from its validated value and service.
    ///
    /// # Arguments
    ///
    /// * `workspace` — Validated workspace that bounds file access.
    /// * `filesystem` — Filesystem service for workspace-scoped operations.
    ///
    /// # Returns
    ///
    /// A [`WorkspaceContext`] containing both workspace resources.
    pub(crate) const fn new(workspace: Workspace, filesystem: FileSystem) -> Self {
        Self {
            workspace,
            filesystem,
        }
    }

    /// Returns the canonical root of the validated workspace.
    ///
    /// # Returns
    ///
    /// A [`Path`] borrowed from the bundled [`Workspace`].
    #[cfg(test)]
    pub(crate) fn root(&self) -> &Path {
        self.workspace.root()
    }
}

/// Returns the workspace resources from the nearest context.
///
/// # Returns
///
/// A [`WorkspaceContext`] cloned from the nearest matching context provider.
///
/// # Panics
///
/// Panics if no [`WorkspaceContext`] is available.
pub(crate) fn use_workspace() -> WorkspaceContext {
    expect_context::<WorkspaceContext>()
}
