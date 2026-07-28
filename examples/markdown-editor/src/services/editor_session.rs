//! Component-facing external editor session coordination.
//!
//! The production session queues editor work through Leptatui's runtime handle
//! so the process receives a restored terminal. Tests can execute the same
//! process boundary immediately without entering a real terminal.

use std::{fmt, path::PathBuf};

use leptatui::prelude::AppHandle;

use super::EditorProcess;

/// Coordinates editor process work with managed terminal ownership.
#[derive(Clone)]
pub(crate) struct EditorSession {
    /// Process boundary that resolves and launches the configured editor.
    process: EditorProcess,
    /// Runtime handle used to release the terminal in production.
    app_handle: Option<AppHandle>,
    /// Whether editor work runs immediately for a headless component test.
    immediate: bool,
}

impl EditorSession {
    /// Creates a session that suspends the managed terminal before editing.
    ///
    /// # Arguments
    ///
    /// * `app_handle` — Active application runtime handle.
    /// * `process` — External editor process boundary.
    ///
    /// # Returns
    ///
    /// An [`EditorSession`] that queues work through `app_handle`.
    pub(crate) fn managed(app_handle: AppHandle, process: EditorProcess) -> Self {
        Self {
            process,
            app_handle: Some(app_handle),
            immediate: false,
        }
    }

    /// Creates a headless session that executes editor work immediately.
    ///
    /// # Arguments
    ///
    /// * `process` — Injected external editor process boundary.
    ///
    /// # Returns
    ///
    /// An [`EditorSession`] suitable for non-terminal component tests.
    #[cfg(test)]
    pub(crate) fn immediate(process: EditorProcess) -> Self {
        Self {
            process,
            app_handle: None,
            immediate: true,
        }
    }

    /// Creates a headless session that retains but does not execute requests.
    ///
    /// # Arguments
    ///
    /// * `process` — External editor process boundary retained by the session.
    ///
    /// # Returns
    ///
    /// An [`EditorSession`] suitable for tests that only inspect key handling.
    #[cfg(test)]
    pub(crate) fn deferred(process: EditorProcess) -> Self {
        Self {
            process,
            app_handle: None,
            immediate: false,
        }
    }

    /// Opens a Markdown path and reports completion to component-owned state.
    ///
    /// Production work runs after terminal restoration. Headless test sessions
    /// invoke the same process and completion callback synchronously.
    ///
    /// # Arguments
    ///
    /// * `path` — Canonical Markdown path supplied to the editor.
    /// * `completion` — Callback receiving the editor result.
    pub(crate) fn edit(
        &self,
        path: PathBuf,
        completion: impl FnOnce(PathBuf, std::io::Result<()>) + 'static,
    ) {
        let process = self.process.clone();
        let task = move || {
            let result = process.edit(&path);
            completion(path, result);
        };

        if self.immediate {
            task();
        } else if let Some(app_handle) = &self.app_handle {
            app_handle.suspend_terminal(task);
        }
    }
}

impl fmt::Debug for EditorSession {
    /// Formats the editor process and execution mode.
    ///
    /// # Arguments
    ///
    /// * `formatter` — Debug formatter receiving the session summary.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating whether formatting succeeded.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EditorSession")
            .field("process", &self.process)
            .field("immediate", &self.immediate)
            .finish_non_exhaustive()
    }
}
