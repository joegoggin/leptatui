//! Viewer route-level component, reload state, and editor synchronization.

use std::path::{Path, PathBuf};

use leptatui::prelude::*;
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

use crate::{contexts::use_notifications, services::is_markdown_path};

use super::{
    components::{ViewerDocument, ViewerDocumentProps},
    style::use_viewer_page_styles,
};

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

/// External-editor failure associated with one requested path.
#[derive(Clone, Debug)]
struct EditorFailure {
    /// Markdown path supplied to the editor.
    path: PathBuf,
    /// Editor launch or exit failure.
    error: String,
}

/// Renders the standalone Markdown viewer and document actions.
///
/// The route identifies the open document. A revision rebuilds the declarative
/// Markdown component after explicit reloads and completed editor sessions.
///
/// # Returns
///
/// A Viewer page component or a current-directory resolution error.
#[component]
pub(crate) fn ViewerPage() -> ViewResult<impl IntoView> {
    let notifications = use_notifications();
    let editor = use_editor();
    let route_params = use_params_map();
    let current_directory = std::env::current_dir()?;
    let document_directory = current_directory.clone();
    let document_path = Memo::new(move |_| {
        let route_path = route_params.get().get("path").map(str::to_owned);
        resolve_viewer_path(route_path.as_deref(), &document_directory)
    });
    let revision = RwSignal::new(0_u64);
    let shortcut_navigate = use_navigate();
    let home_navigate = use_navigate();
    let browse_navigate = use_navigate();
    let open_path = route_params
        .get_untracked()
        .get("path")
        .map_or_else(|| String::from("none"), str::to_owned);
    let editor_failure = RwSignal::new(None::<EditorFailure>);
    let editor_request = RwSignal::new(None::<PathBuf>);

    let completed_editor_status = editor.clone();
    let completed_editor_clear = editor.clone();
    Effect::watch_sync(
        move || completed_editor_status.status(),
        move |status, _, _| {
            let Some(status) = status else {
                return;
            };
            if status == &EditorStatus::Pending {
                return;
            }
            let Some(path) = editor_request.get_untracked() else {
                completed_editor_clear.clear();
                return;
            };
            let failure = match status {
                EditorStatus::Error(error) => {
                    notifications.show_error("Editor failed", error.clone());
                    Some(EditorFailure {
                        path,
                        error: error.clone(),
                    })
                }
                EditorStatus::Complete => {
                    notifications.show_success("Editor closed", "Reloaded the Markdown preview.");
                    None
                }
                EditorStatus::Pending => return,
            };
            editor_failure.set(failure);
            revision.update(|revision| *revision = revision.wrapping_add(1));
            editor_request.set(None);
            completed_editor_clear.clear();
        },
        true,
    );

    let document_key_editor_failure = editor_failure;
    let document = keyed(
        move || {
            let path = document_path.get();
            let editor_error = path.as_ref().ok().and_then(|path| {
                matching_editor_error(&document_key_editor_failure, path.as_deref())
            });
            (path, revision.get(), editor_error)
        },
        move || {
            let (path, route_error) = match document_path.get_untracked() {
                Ok(path) => (path, None),
                Err(error) => (None, Some(error)),
            };
            let editor_error = untrack(|| matching_editor_error(&editor_failure, path.as_deref()));
            view! { <ViewerDocument path=path error=editor_error.or(route_error) /> }.into_view()
        },
    );

    use_viewer_page_styles();

    use_key_event(KeyEventKind::Press, move |key| {
        if key.modifiers != KeyModifiers::NONE {
            return KeyControl::Pass;
        }

        match key.code {
            KeyCode::Char('e') => {
                if let Ok(Some(path)) = document_path.get_untracked() {
                    editor_request.set(Some(path.clone()));
                    editor.edit_file(path);
                } else {
                    editor_failure.set(None);
                }
                KeyControl::Handled
            }
            KeyCode::Char('r') => {
                editor_failure.set(None);
                revision.update(|revision| *revision = revision.wrapping_add(1));
                notifications.show_info("Preview reloaded", "Refreshed the current Markdown file.");
                KeyControl::Handled
            }
            KeyCode::Char('h') | KeyCode::Char('b') => {
                shortcut_navigate("/", NavigateOptions::default());
                KeyControl::Handled
            }
            _ => KeyControl::Pass,
        }
    });

    view! {
        <Div class="viewer-page">
            <Text class="viewer-page__title">"Markdown viewer"</Text>
            <Text class="viewer-page__path">{format!("Open: {open_path}")}</Text>
            {document}
            <Div class="viewer-page__actions">
                <Button on_press=move || {
                    home_navigate("/", NavigateOptions::default());
                    AppControl::Continue
                }>"Home"</Button>
                <Button on_press=move || {
                    browse_navigate("/", NavigateOptions::default());
                    AppControl::Continue
                }>"Browse files"</Button>
            </Div>
            <Text class="viewer-page__help">
                "PgUp/Dn scroll | e edit | r reload | h home | b browse | q quit"
            </Text>
        </Div>
    }
}

/// Resolves and validates the Markdown path represented by a Viewer route.
fn resolve_viewer_path(
    route_path: Option<&str>,
    current_directory: &Path,
) -> Result<Option<PathBuf>, String> {
    let Some(route_path) = route_path.filter(|path| !path.is_empty()) else {
        return Ok(None);
    };
    let route_path = PathBuf::from(route_path);
    let requested = if route_path.is_absolute() {
        route_path
    } else {
        current_directory.join(route_path)
    };
    if !is_markdown_path(&requested) {
        return Err(format!(
            "preview path is not a Markdown file: {}",
            requested.display()
        ));
    }
    Ok(Some(requested))
}

/// Creates an encoded viewer location for an absolute Markdown path.
///
/// # Arguments
///
/// * `path` — Absolute Markdown path to encode.
///
/// # Returns
///
/// A [`String`] containing `/view/` and one encoded absolute path.
pub(crate) fn viewer_location(path: &Path) -> String {
    let path = path.as_os_str().to_string_lossy();
    let encoded = utf8_percent_encode(&path, ROUTE_SEGMENT_ENCODE_SET);
    format!("/view/{encoded}")
}

/// Returns an editor diagnostic only when it belongs to the open path.
fn matching_editor_error(
    editor_failure: &RwSignal<Option<EditorFailure>>,
    path: Option<&Path>,
) -> Option<String> {
    editor_failure.with(|failure| {
        failure
            .as_ref()
            .filter(|failure| Some(failure.path.as_path()) == path)
            .map(|failure| failure.error.clone())
    })
}
