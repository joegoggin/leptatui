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

/// Provides routing, shared styling, contexts, and global controls.
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
        .app-shell => {
            fg: Color::White,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1),
            box_sizing: BoxSizing::BorderBox,
            size: LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::percent(100.0))
            )
        }
        .page => {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::percent(100.0))
            )
        }
        .route-shell => {
            position: Position::Relative,
            size: LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::percent(100.0))
            )
        }
        .page-title => {
            fg: Color::LightCyan,
            modifier: Modifier::BOLD
        }
        .path-context => { fg: Color::LightGreen }
        .page-content => {
            flex_basis: Dimension::from(Length::cells(0.0)),
            flex_grow: 1.0,
            borders: Borders::ALL,
            padding: TuiSpacing::horizontal(1)
        }
        .scroll-content => {
            overflow: Axes::new(Overflow::Hidden, Overflow::Auto)
        }
        .actions => {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            gap: Axes::new(Length::cells(1.0), Length::cells(0.0))
        }
        .section-title => {
            fg: Color::White,
            modifier: Modifier::BOLD
        }
        .directory-entry => { fg: Color::LightBlue }
        .markdown-entry => { fg: Color::White }
        .selected => {
            fg: Color::Black,
            bg: Color::LightCyan,
            modifier: Modifier::BOLD
        }
        .empty => { fg: Color::DarkGray }
        .error => { fg: Color::LightRed }
        .success => { fg: Color::LightGreen }
        .info => { fg: Color::LightCyan }
        .warning => { fg: Color::Yellow }
        .notifications => {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            position: Position::Fixed,
            inset: Edges::new(
                Length::cells(1.0).into(),
                Length::cells(1.0).into(),
                LengthAuto::Auto,
                LengthAuto::Auto
            ),
            z_index: ZIndex::Integer(10)
        }
        .notification => {
            borders: Borders::ALL,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::horizontal(1)
        }
        .help => { fg: Color::Gray }

        @media (max-width: 60) {
            .app-shell => {
                border_type: BorderType::Plain,
                padding: TuiSpacing::ZERO
            }
            .page-content => { padding: TuiSpacing::ZERO }
            .actions => { flex_direction: FlexDirection::Column }
            Button => { padding: TuiSpacing::ZERO }
            TextArea => { padding: TuiSpacing::ZERO }
        }
    }

    view! {
        <Router initial_path=initial_path>
            <Block class="app-shell">
                <Div class="route-shell">
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
