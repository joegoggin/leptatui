//! Routed Markdown editor shell and global controls.

use leptatui::prelude::*;

use crate::{
    contexts::{Notifications, provide_notification_context},
    hooks::{Files, WorkspaceContext},
    pages::{ExplorerPage, HomePage, NotFoundPage, ViewerPage},
    services::EditorSession,
};

use super::style::use_markdown_editor_styles;

/// Provides routing, shell styling, contexts, and global controls.
///
/// # Arguments
///
/// * `workspace` — Validated workspace resources shared by routed pages.
/// * `files` — File-related signals and persistence shared by routed pages.
/// * `editor_session` — Managed external editor handoff service.
/// * `initial_path` — First location for the current TUI session.
///
/// # Returns
///
/// A routed application shell.
#[component]
pub(in crate::app) fn MarkdownEditor(
    workspace: WorkspaceContext,
    files: Files,
    editor_session: EditorSession,
    initial_path: String,
) -> impl IntoView {
    let notifications = provide_notification_context();
    provide_context(workspace);
    provide_context(files.clone());
    provide_context(editor_session);

    if let Some(error) = files.recent_files_error.get_untracked() {
        notifications.show_warning("Recent files unavailable", error.to_string());
    }

    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    use_markdown_editor_styles();

    view! {
        <Router initial_path=initial_path>
            <Block class="markdown-editor">
                <Div class="markdown-editor__routes">
                    <Routes fallback=NotFoundPage>
                        <Route path="/" view=HomePage />
                        <Route path="/files" view=ExplorerPage />
                        <Route path="/view/*path" view=ViewerPage />
                    </Routes>
                    <Notifications />
                </Div>
            </Block>
        </Router>
    }
}
