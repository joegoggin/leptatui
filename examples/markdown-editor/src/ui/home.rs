//! Home page and recent-file controls.

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use leptatui::prelude::*;

use crate::{controller::Controller, domain::RecentFilesState};

use super::{
    shared::{relative_path, routed_page_style},
    viewer::viewer_location,
};

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
pub(super) fn HomePage(controller: Rc<RefCell<Controller>>) -> impl IntoView {
    let shortcut_navigate = use_navigate();
    let button_navigate = use_navigate();

    use_key_event(KeyEventKind::Press, move |key| {
        if key.code == KeyCode::Char('o') && key.modifiers == KeyModifiers::NONE {
            shortcut_navigate("/files", NavigateOptions::default());
            return KeyControl::Handled;
        }

        KeyControl::Pass
    });

    let recent_controller = Rc::clone(&controller);
    let recent_state = recent_controller.borrow().recent_files().clone();
    let root = controller.borrow().workspace().root().to_path_buf();
    let recent_root = root.clone();
    let page_style = routed_page_style();

    view! {
        <Div class="page" style=page_style>
            <Text class="page-title">"Markdown editor"</Text>
            <Text class="path-context">{format!("Root: {}", root.display())}</Text>
            <Div class="actions">
                <Button on_press=move || {
                    button_navigate("/files", NavigateOptions::default());
                    AppControl::Continue
                }>"Open file"</Button>
            </Div>
            <Block class="page-content scroll-content">
                {RecentFilesList::with_props(
                    RecentFilesListProps::builder()
                        .state(recent_state)
                        .root(recent_root)
                        .controller(recent_controller)
                        .build(),
                )}
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
    let navigate = use_navigate();
    let label = relative_path(&root, &path);
    let target = viewer_location(&root, &path);

    view! {
        <Button on_press=move || {
            controller.borrow_mut().open_recent(&path);
            navigate(&target, NavigateOptions::default());
            AppControl::Continue
        }>{label}</Button>
    }
}
