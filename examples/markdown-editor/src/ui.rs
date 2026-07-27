//! Leptatui views and input handling for the Markdown editor.
//!
//! The shell presents safe directory discovery from the application controller.
//! Selection, Markdown preview, responsive panes, and editor controls extend
//! this view in later phases.

use leptatui::prelude::*;

use crate::{
    controller::Controller,
    domain::{ExplorerEntry, ExplorerEntryKind, ExplorerState},
};

/// Creates the root Markdown editor view.
///
/// # Arguments
///
/// * `controller` — Initialized application state and service boundaries.
///
/// # Returns
///
/// A Leptatui view containing the initial application shell.
pub(crate) fn app_view(controller: Controller) -> impl IntoView {
    MarkdownEditor::with_props(
        MarkdownEditorProps::builder()
            .controller(controller)
            .build(),
    )
}

/// Renders the validated workspace root and initial control help.
///
/// # Arguments
///
/// * `controller` — Initialized application state and service boundaries.
///
/// # Returns
///
/// A Leptatui view that exits when the user presses `q`.
#[component]
fn MarkdownEditor(controller: Controller) -> impl IntoView {
    let root = controller.workspace().root().display().to_string();
    let explorer = render_explorer(controller.explorer());

    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    stylesheet! {
        .editor-shell => {
            fg: Color::White,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1)
        }

        .title => {
            fg: Color::LightCyan,
            modifier: Modifier::BOLD
        }

        .root => { fg: Color::LightGreen }
        .directory => { fg: Color::LightCyan }
        .explorer-heading => {
            fg: Color::White,
            modifier: Modifier::BOLD
        }
        .directory-entry => { fg: Color::LightBlue }
        .markdown-entry => { fg: Color::White }
        .empty => { fg: Color::DarkGray }
        .error => { fg: Color::LightRed }
        .help => { fg: Color::Gray }

        @media (max-width: 60) {
            .editor-shell => {
                border_type: BorderType::Plain,
                padding: TuiSpacing::ZERO
            }
        }
    }

    view! {
        <Block class="editor-shell">
            <Div>
                <Text class="title">"Markdown editor"</Text>
                <Text class="root">{format!("Root: {root}")}</Text>
                {explorer}
                <Text class="help">"Selection and preview coming next | q quit"</Text>
            </Div>
        </Block>
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
/// A [`View`] containing the current directory, entries, and optional error.
fn render_explorer(explorer: &ExplorerState) -> AnyView {
    let mut rows = vec![
        text(format!("Directory: {}", explorer.directory().display()))
            .with_classes("directory")
            .into_view(),
        text("Explorer")
            .with_classes("explorer-heading")
            .into_view(),
    ];

    if explorer.entries().is_empty() {
        rows.push(
            text("No directories or Markdown files")
                .with_classes("empty")
                .into_view(),
        );
    } else {
        rows.extend(explorer.entries().iter().map(render_explorer_entry));
    }

    if let Some(error) = explorer.error() {
        rows.push(
            text(format!("Error: {error}"))
                .with_classes("error")
                .into_view(),
        );
    }

    div(rows).with_classes("explorer").into_view()
}

/// Renders one directory or Markdown explorer row.
///
/// # Arguments
///
/// * `entry` — Safe discovered entry to display.
///
/// # Returns
///
/// A [`View`] containing a kind marker and lossy display name.
fn render_explorer_entry(entry: &ExplorerEntry) -> AnyView {
    let (marker, class) = match entry.kind() {
        ExplorerEntryKind::Directory => ("[D]", "directory-entry"),
        ExplorerEntryKind::Markdown => ("[M]", "markdown-entry"),
    };

    text(format!("{marker} {}", entry.name().to_string_lossy()))
        .with_classes(class)
        .into_view()
}
