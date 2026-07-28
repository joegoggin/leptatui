//! Markdown editor initialization, application shell, routes, and styling.

use std::{error::Error, fmt};

use clap::Parser;
use leptatui::prelude::*;

use crate::{
    cli::Cli,
    hooks::{Files, WorkspaceContext},
    pages::{ExplorerPage, HomePage, NotFoundPage, ViewerPage},
    services::{EditorProcess, EditorSession, FileSystem, RecentFilesStore},
};

/// Initializes and renders the prop-free Markdown editor root.
///
/// Startup failures render inside the managed terminal and are retained by the
/// runtime so exiting the diagnostic screen returns the original error.
///
/// # Returns
///
/// An [`AnyView`] containing either the routed editor or a startup diagnostic.
#[component]
pub(crate) fn AppRouter() -> AnyView {
    let app_handle = use_app_handle();

    match initialize() {
        Ok((workspace, files, editor_process)) => view! {
            <MarkdownEditor
                workspace=workspace
                files=files
                editor_session=EditorSession::managed(app_handle, editor_process)
                initial_path=String::from("/")
            />
        },
        Err(error) => {
            let message = error.to_string();
            app_handle.set_exit_error(error);
            view! { <StartupErrorScreen message=message /> }
        }
    }
}

/// Initializes component-owned services, signals, and shared contexts.
///
/// # Returns
///
/// A tuple containing the validated workspace, file state, and editor service.
///
/// # Errors
///
/// Returns [`StartupError`] if CLI parsing, current-directory discovery, or
/// workspace validation fails.
fn initialize() -> std::result::Result<(WorkspaceContext, Files, EditorProcess), StartupError> {
    let cli = Cli::try_parse().map_err(StartupError::new)?;
    let requested_root = cli.requested_root().map_err(StartupError::new)?;
    let filesystem = FileSystem::new();
    let recent_files_store = RecentFilesStore::standard();
    let workspace = filesystem
        .validate_root(&requested_root)
        .map_err(StartupError::new)?;
    let (recent_paths, stored_paths, recent_error) =
        recent_files_store.load_for_workspace(filesystem, &workspace);
    let workspace = WorkspaceContext::new(workspace, filesystem);
    let files = Files::new(recent_paths, stored_paths, recent_error, recent_files_store);

    Ok((workspace, files, EditorProcess::new()))
}

/// Application initialization failure retained through the error screen.
#[derive(Debug)]
struct StartupError {
    /// Displayable startup diagnostic.
    message: String,
    /// Original error preserved for the returned source chain.
    source: Box<dyn Error + Send + Sync>,
}

impl StartupError {
    /// Wraps one application initialization error.
    ///
    /// # Arguments
    ///
    /// * `source` — Original initialization failure.
    ///
    /// # Returns
    ///
    /// A [`StartupError`] retaining the source and its display text.
    fn new<E>(source: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self {
            message: source.to_string(),
            source: Box::new(source),
        }
    }
}

impl fmt::Display for StartupError {
    /// Formats the original initialization diagnostic.
    ///
    /// # Arguments
    ///
    /// * `formatter` — Formatter receiving the diagnostic.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating whether formatting succeeded.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for StartupError {
    /// Returns the original initialization failure.
    ///
    /// # Returns
    ///
    /// An optional error trait object containing the startup source.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Renders an initialization diagnostic until the user quits.
///
/// # Arguments
///
/// * `message` — Startup failure shown in the managed terminal.
///
/// # Returns
///
/// A diagnostic component that exits when the user presses `q`.
#[component]
pub(crate) fn StartupErrorScreen(message: String) -> impl IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    view! {
        <Block>
            <Div>
                <Text>"Markdown editor could not start"</Text>
                <Text>{message}</Text>
                <Text>"Press q to exit."</Text>
            </Div>
        </Block>
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
    provide_context(workspace);
    provide_context(files);
    provide_context(editor_session);

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
        .help => { fg: Color::Gray }

        Button => {
            fg: Color::White,
            borders: Borders::ALL,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::horizontal(1)
        }
        Button:focus => {
            fg: Color::Black,
            bg: Color::LightCyan,
            modifier: Modifier::BOLD,
            border_type: BorderType::Thick
        }

        H1 => { fg: Color::LightCyan }
        H2 => { fg: Color::LightBlue }
        H3 => { fg: Color::LightGreen }
        H4 => { fg: Color::LightYellow }
        H5 => { fg: Color::LightMagenta }
        H6 => { fg: Color::Gray }
        Paragraph => { fg: Color::White }
        OrderedList => { fg: Color::LightCyan }
        UnorderedList => { fg: Color::LightGreen }
        TableHead => { fg: Color::LightCyan }
        CodeBlock => { fg: Color::LightBlue }
        Link:focus => { fg: Color::Black, bg: Color::LightCyan }
        A => { fg: Color::LightBlue }
        A:focus => { fg: Color::Black, bg: Color::LightCyan }
        .active => { fg: Color::LightCyan, modifier: Modifier::BOLD }

        @media (max-width: 60) {
            .app-shell => {
                border_type: BorderType::Plain,
                padding: TuiSpacing::ZERO
            }
            .page-content => { padding: TuiSpacing::ZERO }
            .actions => { flex_direction: FlexDirection::Column }
            Button => { padding: TuiSpacing::ZERO }
        }
    }

    view! {
        <Router initial_path=initial_path>
            <Block class="app-shell">
                <Routes fallback=NotFoundPage>
                    <Route path="/" view=HomePage />
                    <Route path="/files" view=ExplorerPage />
                    <Route path="/view/*path" view=ViewerPage />
                </Routes>
            </Block>
        </Router>
    }
}
