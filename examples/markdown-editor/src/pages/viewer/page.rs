//! Viewer route-level component and route synchronization.

use std::{
    cell::{Cell, RefCell},
    path::Path,
    rc::Rc,
};

use leptatui::prelude::*;
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

use crate::{
    core::Controller,
    pages::shared::{relative_path, routed_page_style},
};

use super::components::{ViewerDocument, ViewerDocumentProps};

/// Characters encoded inside one viewer route path segment.
const ROUTE_SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

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
pub(crate) fn ViewerPage(
    controller: Rc<RefCell<Controller>>,
    edit_requested: Rc<Cell<bool>>,
) -> impl IntoView {
    let shortcut_controller = Rc::clone(&controller);
    let shortcut_navigate = use_navigate();
    let home_navigate = use_navigate();
    let explorer_navigate = use_navigate();
    let route_params = use_params_map();
    let preview_key_controller = Rc::clone(&controller);
    let preview_view_controller = Rc::clone(&controller);
    let preview_key_params = route_params;
    let preview_view_params = route_params;
    let document = keyed(
        move || {
            (
                preview_key_params
                    .get_untracked()
                    .get("path")
                    .unwrap_or_default()
                    .to_owned(),
                preview_key_controller.borrow().preview().revision(),
            )
        },
        move || {
            sync_preview_from_route(&preview_view_controller, preview_view_params);
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
                shortcut_navigate("/", NavigateOptions::default());
                KeyControl::Handled
            }
            KeyCode::Char('b') => {
                shortcut_navigate("/files", NavigateOptions::default());
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
                let open_path = controller
                    .preview()
                    .path()
                    .map_or_else(|| String::from("none"), |path| relative_path(root, path));
                text(format!("Open: {open_path}")).with_classes("path-context")
            }}
            {document}
            <Div class="actions">
                <Button on_press=move || {
                    home_navigate("/", NavigateOptions::default());
                    AppControl::Continue
                }>"Home"</Button>
                <Button on_press=move || {
                    explorer_navigate("/files", NavigateOptions::default());
                    AppControl::Continue
                }>"Browse files"</Button>
            </Div>
            <Text class="help">
                "PgUp/Dn scroll | e edit | r reload | h home | b browse | q quit"
            </Text>
        </Div>
    }
}

/// Creates an encoded viewer location for a workspace Markdown path.
///
/// # Arguments
///
/// * `root` — Canonical workspace root.
/// * `path` — Canonical Markdown path below the root.
///
/// # Returns
///
/// A [`String`] containing `/view/` and encoded relative path segments.
pub(crate) fn viewer_location(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let encoded = relative
        .components()
        .map(|component| {
            utf8_percent_encode(
                &component.as_os_str().to_string_lossy(),
                ROUTE_SEGMENT_ENCODE_SET,
            )
            .to_string()
        })
        .collect::<Vec<_>>()
        .join("/");
    format!("/view/{encoded}")
}

/// Synchronizes controller preview state from the wildcard route parameter.
///
/// # Arguments
///
/// * `controller` — Shared controller to update when the route selects a file.
/// * `params` — Reactive path parameters for the active viewer route.
fn sync_preview_from_route(controller: &Rc<RefCell<Controller>>, params: Memo<ParamsMap>) {
    let Some(relative) = params.get_untracked().get("path").map(str::to_owned) else {
        return;
    };
    let requested = controller.borrow().workspace().root().join(relative);
    if controller.borrow().preview().path() == Some(requested.as_path()) {
        return;
    }
    controller.borrow_mut().open_recent(&requested);
}
