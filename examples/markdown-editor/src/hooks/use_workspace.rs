//! Workspace resources and their required context hook.

#[cfg(test)]
use std::path::Path;

use leptatui::prelude::expect_context;

use crate::services::Workspace;

/// Validated workspace shared by routed components.
#[derive(Clone, Debug)]
pub(crate) struct WorkspaceContext {
    /// Validated workspace that bounds browsing and Markdown access.
    pub(crate) workspace: Workspace,
}

impl WorkspaceContext {
    /// Creates a workspace context from its validated value.
    ///
    /// # Arguments
    ///
    /// * `workspace` — Validated workspace that bounds file access.
    /// # Returns
    ///
    /// A [`WorkspaceContext`] containing the shared workspace.
    pub(crate) const fn new(workspace: Workspace) -> Self {
        Self { workspace }
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
