//! Markdown editor application shell, routes, and global styling.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use leptatui::prelude::*;

use crate::controller::Controller;

use super::{
    explorer::{ExplorerPage, ExplorerPageProps},
    home::{HomePage, HomePageProps},
    not_found::NotFoundPage,
    viewer::{ViewerPage, ViewerPageProps},
};

/// Creates the root Markdown editor view.
///
/// # Arguments
///
/// * `controller` — Shared application state retained across TUI sessions.
/// * `edit_requested` — Shared flag set when the open preview should be edited.
///
/// # Returns
///
/// A routed Leptatui view starting on Home.
#[cfg(test)]
pub(crate) fn app_view(
    controller: Rc<RefCell<Controller>>,
    edit_requested: Rc<Cell<bool>>,
) -> impl View + IntoView {
    app_view_at_path(controller, edit_requested, "/")
}

/// Creates the root Markdown editor view at an explicit path.
///
/// # Arguments
///
/// * `controller` — Shared application state retained across TUI sessions.
/// * `edit_requested` — Shared flag set when the open preview should be edited.
/// * `initial_path` — Location shown when this managed terminal session starts.
///
/// # Returns
///
/// A routed Leptatui view starting on `initial_path`.
pub(crate) fn app_view_at_path(
    controller: Rc<RefCell<Controller>>,
    edit_requested: Rc<Cell<bool>>,
    initial_path: impl Into<String>,
) -> impl View + IntoView {
    MarkdownEditor::with_props(
        MarkdownEditorProps::builder()
            .controller(controller)
            .edit_requested(edit_requested)
            .initial_path(initial_path.into())
            .build(),
    )
}

/// Provides routing, shared styling, and global application controls.
///
/// # Arguments
///
/// * `controller` — Shared application state.
/// * `edit_requested` — Flag used to request a restored-terminal edit.
/// * `initial_path` — First location for the current TUI session.
///
/// # Returns
///
/// A routed application shell.
#[component]
fn MarkdownEditor(
    controller: Rc<RefCell<Controller>>,
    edit_requested: Rc<Cell<bool>>,
    initial_path: String,
) -> impl IntoView {
    let home_controller = Rc::clone(&controller);
    let explorer_controller = Rc::clone(&controller);
    let viewer_controller = Rc::clone(&controller);
    let viewer_edit_requested = Rc::clone(&edit_requested);

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
                    <Route
                        path="/"
                        view=move || {
                            HomePage::with_props(
                                HomePageProps::builder()
                                    .controller(Rc::clone(&home_controller))
                                    .build(),
                            )
                        }
                    />
                    <Route
                        path="/files"
                        view=move || {
                            ExplorerPage::with_props(
                                ExplorerPageProps::builder()
                                    .controller(Rc::clone(&explorer_controller))
                                    .build(),
                            )
                        }
                    />
                    <Route
                        path="/view/*path"
                        view=move || {
                            ViewerPage::with_props(
                                ViewerPageProps::builder()
                                    .controller(Rc::clone(&viewer_controller))
                                    .edit_requested(Rc::clone(&viewer_edit_requested))
                                    .build(),
                            )
                        }
                    />
                </Routes>
            </Block>
        </Router>
    }
}
