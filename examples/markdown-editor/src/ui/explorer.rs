//! Workspace file explorer page and listing components.

use std::{cell::RefCell, rc::Rc};

use leptatui::prelude::*;

use crate::{
    controller::{Controller, ExplorerActivation},
    domain::{ExplorerEntry, ExplorerEntryKind, ExplorerState},
};

use super::{
    shared::{relative_path, routed_page_style},
    viewer::viewer_location,
};

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
pub(super) fn ExplorerPage(controller: Rc<RefCell<Controller>>) -> impl IntoView {
    let shortcut_controller = Rc::clone(&controller);
    let shortcut_navigate = use_navigate();

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
                    let target = {
                        let controller = shortcut_controller.borrow();
                        controller
                            .preview()
                            .path()
                            .map(|path| viewer_location(controller.workspace().root(), path))
                    };
                    if let Some(target) = target {
                        shortcut_navigate(&target, NavigateOptions::default());
                    }
                }
                KeyControl::Handled
            }
            KeyCode::Left | KeyCode::Char('h') => {
                shortcut_controller.borrow_mut().browse_parent();
                KeyControl::Handled
            }
            KeyCode::Esc => {
                shortcut_navigate("/", NavigateOptions::default());
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
    let home_navigate = use_navigate();
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
                    home_navigate("/", NavigateOptions::default());
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
