//! Routed Leptatui pages and input handling for the Markdown editor.
//!
//! The root component provides typed route state while Home, Explorer, and
//! Viewer components own their page-specific controls. Shared controller state
//! survives route changes and restored-terminal editor sessions.

use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    rc::Rc,
};

use leptatui::prelude::*;

use crate::{
    controller::{Controller, ExplorerActivation},
    domain::{ExplorerEntry, ExplorerEntryKind, ExplorerState, PreviewState, RecentFilesState},
};

/// Pages available in the Markdown editor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AppRoute {
    /// Landing page with open and recent-file actions.
    Home,
    /// Workspace file browser.
    Explorer,
    /// Open Markdown document.
    Viewer,
}

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
    app_view_at_route(controller, edit_requested, AppRoute::Home)
}

/// Creates the root Markdown editor view at an explicit route.
///
/// # Arguments
///
/// * `controller` — Shared application state retained across TUI sessions.
/// * `edit_requested` — Shared flag set when the open preview should be edited.
/// * `initial_route` — Page shown when this managed terminal session starts.
///
/// # Returns
///
/// A routed Leptatui view starting on `initial_route`.
pub(crate) fn app_view_at_route(
    controller: Rc<RefCell<Controller>>,
    edit_requested: Rc<Cell<bool>>,
    initial_route: AppRoute,
) -> impl View + IntoView {
    MarkdownEditor::with_props(
        MarkdownEditorProps::builder()
            .controller(controller)
            .edit_requested(edit_requested)
            .initial_route(initial_route)
            .build(),
    )
}

/// Provides routing, shared styling, and global application controls.
///
/// # Arguments
///
/// * `controller` — Shared application state.
/// * `edit_requested` — Flag used to request a restored-terminal edit.
/// * `initial_route` — First page for the current TUI session.
///
/// # Returns
///
/// A routed application shell.
#[component]
fn MarkdownEditor(
    controller: Rc<RefCell<Controller>>,
    edit_requested: Rc<Cell<bool>>,
    initial_route: AppRoute,
) -> impl IntoView {
    let route_state = provide_route(initial_route);
    let route = route_state.route();
    let home_controller = Rc::clone(&controller);
    let home_page = keyed(
        || (),
        move || {
            HomePage::with_props(
                HomePageProps::builder()
                    .controller(Rc::clone(&home_controller))
                    .build(),
            )
        },
    );
    let explorer_controller = Rc::clone(&controller);
    let explorer_page = keyed(
        || (),
        move || {
            ExplorerPage::with_props(
                ExplorerPageProps::builder()
                    .controller(Rc::clone(&explorer_controller))
                    .build(),
            )
        },
    );
    let viewer_controller = Rc::clone(&controller);
    let viewer_edit_requested = Rc::clone(&edit_requested);
    let viewer_page = keyed(
        || (),
        move || {
            ViewerPage::with_props(
                ViewerPageProps::builder()
                    .controller(Rc::clone(&viewer_controller))
                    .edit_requested(Rc::clone(&viewer_edit_requested))
                    .build(),
            )
        },
    );

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
        <Block class="app-shell">
            {move || {
                match route.get_untracked() {
                    AppRoute::Home => home_page.clone().into_view(),
                    AppRoute::Explorer => explorer_page.clone().into_view(),
                    AppRoute::Viewer => viewer_page.clone().into_view(),
                }
            }}
        </Block>
    }
}

/// Renders the landing page and its recent-file actions.
///
/// # Arguments
///
/// * `controller` — Shared application state.
///
/// # Returns
///
/// A Home page component.
#[component]
fn HomePage(controller: Rc<RefCell<Controller>>) -> impl IntoView {
    let shortcut_navigate = use_navigate::<AppRoute>();
    let button_navigate = use_navigate::<AppRoute>();

    use_key_event(KeyEventKind::Press, move |key| {
        if key.code == KeyCode::Char('o') && key.modifiers == KeyModifiers::NONE {
            shortcut_navigate.set(AppRoute::Explorer);
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    let recent_controller = Rc::clone(&controller);
    let root = controller.borrow().workspace().root().to_path_buf();
    let recent_root = root.clone();
    let page_style = routed_page_style();

    view! {
        <Div class="page" style=page_style>
            <Text class="page-title">"Markdown editor"</Text>
            <Text class="path-context">{format!("Root: {}", root.display())}</Text>
            <Div class="actions">
                <Button on_press=move || {
                    button_navigate.set(AppRoute::Explorer);
                    AppControl::Continue
                }>"Open file"</Button>
            </Div>
            <Block class="page-content scroll-content">
                {move || {
                    RecentFilesList::with_props(
                        RecentFilesListProps::builder()
                            .state(recent_controller.borrow().recent_files().clone())
                            .root(recent_root.clone())
                            .controller(Rc::clone(&recent_controller))
                            .build(),
                    )
                }}
            </Block>
            <Text class="help">"o open file | Tab/Enter actions | q quit"</Text>
        </Div>
    }
}

/// Renders the recent-file section on Home.
///
/// # Arguments
///
/// * `state` — Recent paths and persistence error.
/// * `root` — Active workspace root.
/// * `controller` — Shared application state used to open a recent path.
///
/// # Returns
///
/// A recent-file list with an empty state or warning when applicable.
#[component]
fn RecentFilesList(
    state: RecentFilesState,
    root: PathBuf,
    controller: Rc<RefCell<Controller>>,
) -> impl IntoView {
    let mut rows = vec![
        text("Recent files")
            .with_classes("section-title")
            .into_view(),
    ];

    if state.entries().is_empty() {
        rows.push(
            text("No recent Markdown files")
                .with_classes("empty")
                .into_view(),
        );
    } else {
        rows.extend(state.entries().iter().cloned().map(|path| {
            RecentFileEntry::with_props(
                RecentFileEntryProps::builder()
                    .path(path)
                    .root(root.clone())
                    .controller(Rc::clone(&controller))
                    .build(),
            )
            .into_view()
        }));
    }

    if let Some(error) = state.error() {
        rows.push(
            text(format!("Recent files warning: {error}"))
                .with_classes("error")
                .into_view(),
        );
    }

    div(rows)
}

/// Renders one actionable recent-file row.
///
/// # Arguments
///
/// * `path` — Canonical recent Markdown path.
/// * `root` — Active workspace root.
/// * `controller` — Shared application state.
///
/// # Returns
///
/// A button that opens `path` in Viewer.
#[component]
fn RecentFileEntry(
    path: PathBuf,
    root: PathBuf,
    controller: Rc<RefCell<Controller>>,
) -> impl IntoView {
    let navigate = use_navigate::<AppRoute>();
    let label = relative_path(&root, &path);

    view! {
        <Button on_press=move || {
            controller.borrow_mut().open_recent(&path);
            navigate.set(AppRoute::Viewer);
            AppControl::Continue
        }>{label}</Button>
    }
}

/// Renders the standalone workspace file explorer.
///
/// # Arguments
///
/// * `controller` — Shared application state.
///
/// # Returns
///
/// An Explorer page component.
#[component]
fn ExplorerPage(controller: Rc<RefCell<Controller>>) -> impl IntoView {
    let shortcut_controller = Rc::clone(&controller);
    let shortcut_navigate = use_navigate::<AppRoute>();

    use_key_event(KeyEventKind::Press, move |key| {
        if key.modifiers != KeyModifiers::NONE {
            return KeyControl::Pass;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                shortcut_controller.borrow_mut().select_previous();
                KeyControl::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                shortcut_controller.borrow_mut().select_next();
                KeyControl::Handled
            }
            KeyCode::Enter => {
                if shortcut_controller.borrow_mut().activate_selected()
                    == ExplorerActivation::Document
                {
                    shortcut_navigate.set(AppRoute::Viewer);
                }
                KeyControl::Handled
            }
            KeyCode::Left | KeyCode::Char('h') => {
                shortcut_controller.borrow_mut().browse_parent();
                KeyControl::Handled
            }
            KeyCode::Esc => {
                shortcut_navigate.set(AppRoute::Home);
                KeyControl::Handled
            }
            _ => KeyControl::Pass,
        }
    });

    ExplorerContent::with_props(
        ExplorerContentProps::builder()
            .controller(controller)
            .build(),
    )
}

/// Renders the current explorer state inside a stable scroll boundary.
///
/// # Arguments
///
/// * `controller` — Shared application state.
///
/// # Returns
///
/// Explorer headings and the scrollable listing.
#[component]
fn ExplorerContent(controller: Rc<RefCell<Controller>>) -> impl IntoView {
    let home_navigate = use_navigate::<AppRoute>();
    let root = controller.borrow().workspace().root().to_path_buf();
    let directory_controller = Rc::clone(&controller);
    let list_controller = Rc::clone(&controller);
    let page_style = routed_page_style();

    view! {
        <Div class="page" style=page_style>
            <Text class="page-title">"File explorer"</Text>
            <Text class="path-context">{format!("Root: {}", root.display())}</Text>
            {move || {
                let controller = directory_controller.borrow();
                let directory = relative_path(
                    controller.workspace().root(),
                    controller.explorer().directory(),
                );
                text(format!("Directory: {directory}")).with_classes("path-context")
            }}
            <Block class="page-content scroll-content">
                {move || {
                    ExplorerList::with_props(
                        ExplorerListProps::builder()
                            .state(list_controller.borrow().explorer().clone())
                            .build(),
                    )
                }}
            </Block>
            <Div class="actions">
                <Button on_press=move || {
                    home_navigate.set(AppRoute::Home);
                    AppControl::Continue
                }>"Home"</Button>
            </Div>
            <Text class="help">
                "↑/k ↓/j select | Enter open | ←/h parent | Esc home | q quit"
            </Text>
        </Div>
    }
}

/// Renders explorer rows and any recoverable directory error.
///
/// # Arguments
///
/// * `state` — Current explorer snapshot.
///
/// # Returns
///
/// A directory listing component.
#[component]
fn ExplorerList(state: ExplorerState) -> impl IntoView {
    let mut rows = Vec::new();

    if state.entries().is_empty() {
        rows.push(
            text("No directories or Markdown files")
                .with_classes("empty")
                .into_view(),
        );
    } else {
        rows.extend(
            state
                .entries()
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, entry)| {
                    ExplorerEntryRow::with_props(
                        ExplorerEntryRowProps::builder()
                            .entry(entry)
                            .selected(state.selection() == Some(index))
                            .build(),
                    )
                    .into_view()
                }),
        );
    }

    if let Some(error) = state.error() {
        rows.push(
            text(format!("Error: {error}"))
                .with_classes("error")
                .into_view(),
        );
    }

    div(rows)
}

/// Renders one selected or unselected explorer entry.
///
/// # Arguments
///
/// * `entry` — Safe discovered filesystem entry.
/// * `selected` — Whether the entry is highlighted.
///
/// # Returns
///
/// A styled explorer row.
#[component]
fn ExplorerEntryRow(entry: ExplorerEntry, selected: bool) -> impl IntoView {
    let (marker, class) = match entry.kind() {
        ExplorerEntryKind::Directory => ("[D]", "directory-entry"),
        ExplorerEntryKind::Markdown => ("[M]", "markdown-entry"),
    };
    let selection_marker = if selected { ">" } else { " " };
    let classes = if selected {
        format!("{class} selected")
    } else {
        String::from(class)
    };

    text(format!(
        "{selection_marker} {marker} {}",
        entry.name().to_string_lossy()
    ))
    .with_classes(classes)
}

/// Renders the standalone Markdown viewer and document actions.
///
/// # Arguments
///
/// * `controller` — Shared application state.
/// * `edit_requested` — Flag used to leave the managed TUI before editing.
///
/// # Returns
///
/// A Viewer page component.
#[component]
fn ViewerPage(
    controller: Rc<RefCell<Controller>>,
    edit_requested: Rc<Cell<bool>>,
) -> impl IntoView {
    let shortcut_controller = Rc::clone(&controller);
    let shortcut_navigate = use_navigate::<AppRoute>();
    let home_navigate = use_navigate::<AppRoute>();
    let explorer_navigate = use_navigate::<AppRoute>();
    let preview_key_controller = Rc::clone(&controller);
    let preview_view_controller = Rc::clone(&controller);
    let document = keyed(
        move || preview_key_controller.borrow().preview().revision(),
        move || {
            ViewerDocument::with_props(
                ViewerDocumentProps::builder()
                    .preview(preview_view_controller.borrow().preview().clone())
                    .build(),
            )
        },
    );
    let page_style = routed_page_style();

    use_key_event(KeyEventKind::Press, move |key| {
        if key.modifiers != KeyModifiers::NONE {
            return KeyControl::Pass;
        }

        match key.code {
            KeyCode::Char('e') => {
                if shortcut_controller.borrow().preview().path().is_some() {
                    edit_requested.set(true);
                    KeyControl::Exit
                } else {
                    KeyControl::Handled
                }
            }
            KeyCode::Char('r') => {
                shortcut_controller.borrow_mut().reload_preview();
                KeyControl::Handled
            }
            KeyCode::Char('h') => {
                shortcut_navigate.set(AppRoute::Home);
                KeyControl::Handled
            }
            KeyCode::Char('b') => {
                shortcut_navigate.set(AppRoute::Explorer);
                KeyControl::Handled
            }
            _ => KeyControl::Pass,
        }
    });

    view! {
        <Div class="page" style=page_style>
            <Text class="page-title">"Markdown viewer"</Text>
            {move || {
                let controller = controller.borrow();
                let root = controller.workspace().root();
                let open_path = controller.preview().path().map_or_else(
                    || String::from("none"),
                    |path| relative_path(root, path),
                );
                text(format!("Open: {open_path}")).with_classes("path-context")
            }}
            {document}
            <Div class="actions">
                <Button on_press=move || {
                    home_navigate.set(AppRoute::Home);
                    AppControl::Continue
                }>"Home"</Button>
                <Button on_press=move || {
                    explorer_navigate.set(AppRoute::Explorer);
                    AppControl::Continue
                }>"Browse files"</Button>
            </Div>
            <Text class="help">
                "PgUp/Dn scroll | e edit | r reload | h home | b browse | q quit"
            </Text>
        </Div>
    }
}

/// Renders an open Markdown source or its recoverable error.
///
/// # Arguments
///
/// * `preview` — Controller-owned preview snapshot.
///
/// # Returns
///
/// A semantic Markdown document, error, or empty hint.
#[component]
fn ViewerDocument(preview: PreviewState) -> impl IntoView {
    let body = if let Some(source) = preview.source() {
        markdown_with_options(
            source,
            MarkdownOptions::default()
                .syntax_theme(SyntaxTheme::Dark)
                .line_numbers(true),
        )
    } else if let Some(error) = preview.error() {
        text(format!("Error: {error}"))
            .with_classes("error")
            .into_view()
    } else {
        text("Choose a Markdown file from Home or Explorer")
            .with_classes("empty")
            .into_view()
    };
    let content_style = TuiStyle::new()
        .flex_basis(Dimension::from(Length::cells(0.0)))
        .flex_grow(1.0)
        .borders(Borders::ALL)
        .padding(TuiSpacing::horizontal(1))
        .overflow(Axes::new(Overflow::Hidden, Overflow::Auto));

    view! {
        <Block style=content_style>{body}</Block>
    }
}

/// Formats a workspace path relative to its root.
///
/// # Arguments
///
/// * `root` — Canonical workspace root.
/// * `path` — Canonical directory or Markdown path to display.
///
/// # Returns
///
/// A [`String`] containing `.`, a relative workspace path, or the original
/// absolute path when it is not below `root`.
fn relative_path(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(relative) if relative.as_os_str().is_empty() => String::from("."),
        Ok(relative) => relative.display().to_string(),
        Err(_) => path.display().to_string(),
    }
}

/// Returns the full-size column layout shared by routed pages.
///
/// # Returns
///
/// A [`TuiStyle`] that makes page content participate in the application
/// shell's available size.
fn routed_page_style() -> TuiStyle {
    TuiStyle::new()
        .display(Display::Flex)
        .flex_direction(FlexDirection::Column)
        .size(LayoutSize::new(
            Dimension::from(Length::percent(100.0)),
            Dimension::from(Length::percent(100.0)),
        ))
}
