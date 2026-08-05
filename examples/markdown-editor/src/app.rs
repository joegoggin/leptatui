//! Markdown editor application entry components.
//!
//! # Modules
//!
//! - [`markdown_editor`] — Routed application shell and its stylesheet.

mod markdown_editor;

use leptatui::prelude::*;

use crate::{
    hooks::{Files, WorkspaceContext},
    services::{EditorProcess, EditorSession},
};

use self::markdown_editor::{MarkdownEditor, MarkdownEditorProps};

/// Renders the initialized Markdown editor root.
///
/// # Arguments
///
/// * `workspace` — Validated workspace resources shared by routed pages.
/// * `files` — File-related signals and persistence shared by routed pages.
/// * `editor_process` — External editor process service.
///
/// # Returns
///
/// An [`AnyView`] containing the routed editor.
#[component]
pub(crate) fn AppRouter(
    workspace: WorkspaceContext,
    files: Files,
    editor_process: EditorProcess,
) -> AnyView {
    let app_handle = use_app_handle();

    view! {
        <MarkdownEditor
            workspace=workspace
            files=files
            editor_session=EditorSession::managed(app_handle, editor_process)
            initial_path=String::from("/")
        />
    }
}

/// Creates the root Markdown editor view.
///
/// # Arguments
///
/// * `workspace` — Validated workspace resources shared by routed pages.
/// * `files` — File-related signals and persistence shared by routed pages.
///
/// # Returns
///
/// A routed Leptatui view starting on Home.
#[cfg(test)]
pub(crate) fn app_view(workspace: WorkspaceContext, files: Files) -> AnyView {
    app_view_at_path(
        workspace,
        files,
        EditorSession::deferred(EditorProcess::new()),
        "/",
    )
}

/// Creates the root Markdown editor view at an explicit path.
///
/// # Arguments
///
/// * `workspace` — Validated workspace resources shared by routed pages.
/// * `files` — File-related signals and persistence shared by routed pages.
/// * `editor_session` — Editor handoff service used by the Viewer.
/// * `initial_path` — Location shown when the managed session starts.
///
/// # Returns
///
/// A routed Leptatui view starting on `initial_path`.
#[cfg(test)]
pub(crate) fn app_view_at_path(
    workspace: WorkspaceContext,
    files: Files,
    editor_session: EditorSession,
    initial_path: impl Into<String>,
) -> AnyView {
    let initial_path = initial_path.into();

    view! {
        <MarkdownEditor
            workspace=workspace
            files=files
            editor_session=editor_session
            initial_path=initial_path
        />
    }
}
