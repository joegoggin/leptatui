//! Markdown editor application shell, shared values, routes, and styling.

use leptatui::prelude::*;

use crate::{
    hooks::{Files, WorkspaceContext},
    pages::{ExplorerPage, HomePage, NotFoundPage, ViewerPage},
};

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
    app_view_at_path(workspace, files, "/")
}

/// Creates the root Markdown editor view at an explicit path.
///
/// # Arguments
///
/// * `workspace` — Validated workspace resources shared by routed pages.
/// * `files` — File-related signals and persistence shared by routed pages.
/// * `initial_path` — Location shown when the managed session starts.
///
/// # Returns
///
/// A routed Leptatui view starting on `initial_path`.
pub(crate) fn app_view_at_path(
    workspace: WorkspaceContext,
    files: Files,
    initial_path: impl Into<String>,
) -> AnyView {
    let initial_path = initial_path.into();

    view! {
        <MarkdownEditor
            workspace=workspace
            files=files
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
/// * `initial_path` — First location for the current TUI session.
///
/// # Returns
///
/// A routed application shell.
#[component]
fn MarkdownEditor(
    workspace: WorkspaceContext,
    files: Files,
    initial_path: String,
) -> impl IntoView {
    provide_context(workspace);
    provide_context(files);

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
