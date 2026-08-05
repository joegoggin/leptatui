//! Markdown editor application shell, routes, and styling.

use leptatui::prelude::*;

use crate::{
    contexts::{Notifications, provide_notification_context},
    hooks::{Files, WorkspaceContext},
    pages::{ExplorerPage, HomePage, NotFoundPage, ViewerPage},
    services::{EditorProcess, EditorSession},
};

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
fn MarkdownEditor(
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

    stylesheet! {
        .markdown-editor => {
            fg: Color::White,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1),
            box_sizing: BoxSizing::BorderBox,
            size: LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::percent(100.0))
            )

            @media (max-width: 60) {
                border_type: BorderType::Plain,
                padding: TuiSpacing::ZERO
            }

            &__routes => {
                position: Position::Relative,
                size: LayoutSize::new(
                    Dimension::from(Length::percent(100.0)),
                    Dimension::from(Length::percent(100.0))
                )
            }
        }
    }

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
