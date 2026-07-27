//! Leptatui views and input handling for the Markdown editor.
//!
//! The application keeps explorer navigation in shared controller state and
//! rebuilds its selected row, Markdown preview, errors, and responsive pane
//! layout after keyboard commands.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use leptatui::prelude::*;

use crate::{
    controller::Controller,
    domain::{ExplorerEntry, ExplorerEntryKind, ExplorerState, PreviewState},
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
/// A Leptatui view containing the interactive Markdown editor.
pub(crate) fn app_view(
    controller: Rc<RefCell<Controller>>,
    edit_requested: Rc<Cell<bool>>,
) -> impl View + IntoView {
    MarkdownEditor::with_props(
        MarkdownEditorProps::builder()
            .controller(controller)
            .edit_requested(edit_requested)
            .build(),
    )
}

/// Renders interactive explorer and preview panes.
///
/// # Arguments
///
/// * `controller` — Shared application state retained across TUI sessions.
/// * `edit_requested` — Shared flag set when the open preview should be edited.
///
/// # Returns
///
/// A responsive view that handles explorer, preview, reload, and exit commands.
#[component]
fn MarkdownEditor(
    controller: Rc<RefCell<Controller>>,
    edit_requested: Rc<Cell<bool>>,
) -> impl IntoView {
    let event_controller = Rc::clone(&controller);
    let preview_key_controller = Rc::clone(&controller);
    let preview_view_controller = Rc::clone(&controller);
    let preview = keyed(
        move || preview_key_controller.borrow().preview().revision(),
        move || render_preview(preview_view_controller.borrow().preview()),
    );

    use_key_event(KeyEventKind::Press, move |key| {
        let plain_key = key.modifiers == KeyModifiers::NONE;

        match key.code {
            KeyCode::Char('q') if plain_key => KeyControl::Exit,
            KeyCode::Char('e') if plain_key => {
                if event_controller.borrow().preview().path().is_some() {
                    edit_requested.set(true);
                    KeyControl::Exit
                } else {
                    KeyControl::Handled
                }
            }
            KeyCode::Up | KeyCode::Char('k') if plain_key => {
                event_controller.borrow_mut().select_previous();
                KeyControl::Handled
            }
            KeyCode::Down | KeyCode::Char('j') if plain_key => {
                event_controller.borrow_mut().select_next();
                KeyControl::Handled
            }
            KeyCode::Enter if plain_key => {
                event_controller.borrow_mut().activate_selected();
                KeyControl::Handled
            }
            KeyCode::Left | KeyCode::Char('h') if plain_key => {
                event_controller.borrow_mut().browse_parent();
                KeyControl::Handled
            }
            KeyCode::Char('r') if plain_key => {
                event_controller.borrow_mut().reload_preview();
                KeyControl::Handled
            }
            _ => KeyControl::Pass,
        }
    });

    stylesheet! {
        .editor-shell => {
            fg: Color::White,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1),
            box_sizing: BoxSizing::BorderBox,
            size: LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::percent(100.0))
            )
        }
        .editor-layout => {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::percent(100.0))
            )
        }
        .title => {
            fg: Color::LightCyan,
            modifier: Modifier::BOLD
        }
        .path-context => { fg: Color::LightGreen }
        .workspace => {
            display: Display::Flex,
            flex_direction: FlexDirection::RowReverse,
            flex_basis: Dimension::from(Length::cells(0.0)),
            flex_grow: 1.0,
            gap: Axes::new(Length::cells(1.0), Length::cells(0.0)),
            align_items: AlignItems::Stretch
        }
        .explorer-pane => {
            flex_basis: Dimension::from(Length::cells(30.0)),
            flex_shrink: 0.0,
            borders: Borders::ALL,
            padding: TuiSpacing::horizontal(1)
        }
        .preview-pane => {
            flex_basis: Dimension::from(Length::cells(0.0)),
            flex_grow: 1.0,
            borders: Borders::ALL,
            padding: TuiSpacing::horizontal(1),
            overflow: Axes::new(Overflow::Hidden, Overflow::Auto)
        }
        .pane-heading => {
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
            .editor-shell => {
                border_type: BorderType::Plain,
                padding: TuiSpacing::ZERO
            }
            .workspace => {
                flex_direction: FlexDirection::ColumnReverse,
                gap: Axes::new(Length::cells(0.0), Length::cells(1.0))
            }
            .explorer-pane => {
                flex_basis: Dimension::from(Length::cells(9.0)),
                flex_shrink: 1.0,
                padding: TuiSpacing::ZERO
            }
            .preview-pane => {
                flex_basis: Dimension::from(Length::cells(0.0)),
                padding: TuiSpacing::ZERO
            }
        }
    }

    view! {
        <Block class="editor-shell">
            {move || render_editor(&controller.borrow(), preview.clone())}
        </Block>
    }
}

/// Renders the current controller snapshot.
///
/// # Arguments
///
/// * `controller` — Controller state read from the shared session.
/// * `preview` — Keyed Markdown preview retained until its revision changes.
///
/// # Returns
///
/// An [`AnyView`] containing path context, responsive panes, and control help.
fn render_editor(controller: &Controller, preview: DynamicView) -> AnyView {
    let root = controller.workspace().root();
    let directory = relative_path(root, controller.explorer().directory());
    let open_path = controller
        .preview()
        .path()
        .map_or_else(|| String::from("none"), |path| relative_path(root, path));
    let explorer = render_explorer(controller.explorer());

    view! {
        <Div class="editor-layout">
            <Text class="title">"Markdown editor"</Text>
            <Text class="path-context">{format!("Root: {}", root.display())}</Text>
            <Text class="path-context">{format!("Directory: {directory}")}</Text>
            <Text class="path-context">{format!("Open: {open_path}")}</Text>
            <Div class="workspace">
                <Block class="preview-pane">{preview}</Block>
                <Block class="explorer-pane">{explorer}</Block>
            </Div>
            <Text class="help">
                "↑/k ↓/j | Enter open | ←/h parent | PgUp/Dn | e edit | r reload | q quit"
            </Text>
        </Div>
    }
    .into_view()
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
fn relative_path(root: &std::path::Path, path: &std::path::Path) -> String {
    match path.strip_prefix(root) {
        Ok(relative) if relative.as_os_str().is_empty() => String::from("."),
        Ok(relative) => relative.display().to_string(),
        Err(_) => path.display().to_string(),
    }
}

/// Renders the current directory listing and recoverable error state.
///
/// # Arguments
///
/// * `explorer` — Controller-owned explorer state to present.
///
/// # Returns
///
/// A [`View`] containing selected entries and an optional navigation error.
fn render_explorer(explorer: &ExplorerState) -> AnyView {
    let mut rows = vec![text("Explorer").with_classes("pane-heading").into_view()];

    if explorer.entries().is_empty() {
        rows.push(
            text("No directories or Markdown files")
                .with_classes("empty")
                .into_view(),
        );
    } else {
        rows.extend(explorer.entries().iter().enumerate().map(|(index, entry)| {
            render_explorer_entry(entry, explorer.selection() == Some(index))
        }));
    }

    if let Some(error) = explorer.error() {
        rows.push(
            text(format!("Error: {error}"))
                .with_classes("error")
                .into_view(),
        );
    }

    div(rows).into_view()
}

/// Renders one directory or Markdown explorer row.
///
/// # Arguments
///
/// * `entry` — Safe discovered entry to display.
/// * `selected` — Whether this row is currently highlighted.
///
/// # Returns
///
/// A [`View`] containing a selection marker, kind marker, and display name.
fn render_explorer_entry(entry: &ExplorerEntry, selected: bool) -> AnyView {
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
    .into_view()
}

/// Renders the open Markdown source or its recoverable error.
///
/// # Arguments
///
/// * `preview` — Controller-owned preview state to present.
///
/// # Returns
///
/// A [`View`] containing a semantic Markdown document, error, or empty hint.
fn render_preview(preview: &PreviewState) -> AnyView {
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
        text("Select a Markdown file and press Enter")
            .with_classes("empty")
            .into_view()
    };

    div((
        text("Preview").with_classes("pane-heading").into_view(),
        body,
    ))
    .into_view()
}
