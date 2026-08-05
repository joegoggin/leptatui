//! Viewer route-level component, local reload state, and shared file synchronization.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use leptatui::prelude::*;
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

use crate::{
    contexts::use_notifications,
    services::{EditorSession, RecentFilesStore, is_markdown_path, volume_root},
};

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
    /// Shared editor launch or exit failure.
    error: Arc<anyhow::Error>,
}

/// Renders the standalone Markdown viewer and document actions.
///
/// The route identifies the open document. Only the reload revision belongs
/// to this page instance; recent files and editor failures use shared signals.
///
/// # Returns
///
/// A Viewer page component or a filesystem initialization error.
#[component]
pub(crate) fn ViewerPage() -> ViewResult<impl IntoView> {
    let notifications = use_notifications();
    let editor_session = expect_context::<EditorSession>();
    let recent_files_store = expect_context::<RecentFilesStore>();
    let route_params = use_params_map();
    let initial_path = route_params
        .get_untracked()
        .get("path")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or(std::env::current_dir()?);
    let filesystem = use_file_system(volume_root(&initial_path))?;
    let revision = RwSignal::new(0_u64);
    let shortcut_navigate = use_navigate();
    let home_navigate = use_navigate();
    let file_selector = use_file_selector();
    let shortcut_file_selector = file_selector.clone();
    let button_file_selector = file_selector.clone();
    let selected_navigate = use_navigate();
    let open_path = route_params
        .get_untracked()
        .get("path")
        .map_or_else(|| String::from("none"), str::to_owned);
    let editor_failure = RwSignal::new(None::<EditorFailure>);
    let document_path = RwSignal::new(None::<PathBuf>);
    let load_error = RwSignal::new(None::<String>);
    let load_generation = RwSignal::new(0_u64);
    let read_document = ArcRwSignal::new(None::<FileOperation<String>>);

    Effect::new(move || {
        if let Some(file) = file_selector.get_file() {
            selected_navigate(&viewer_location(&file), NavigateOptions::default());
        }
    });
    let read_for_route = read_document.clone();
    let route_filesystem = filesystem.clone();
    let route_current_directory = std::env::current_dir()?;
    Effect::watch_sync(
        move || {
            (
                route_params
                    .get()
                    .get("path")
                    .unwrap_or_default()
                    .to_owned(),
                revision.get(),
            )
        },
        move |(route_path, _), _, _| {
            let _ = load_generation.try_update(|generation| {
                *generation = generation.wrapping_add(1);
            });
            let _ = document_path.try_set(None);
            let _ = load_error.try_set(None);
            let _ = read_for_route.try_set(None);
            if route_path.is_empty() {
                return;
            }
            let route_path = PathBuf::from(route_path);
            let requested = if route_path.is_absolute() {
                route_path
            } else {
                route_current_directory.join(route_path)
            };
            if !is_markdown_path(&requested) {
                let _ = load_error.try_set(Some(format!(
                    "preview path is not a Markdown file: {}",
                    requested.display()
                )));
                return;
            }
            let _ = document_path.try_set(Some(requested.clone()));
            let _ = read_for_route.try_set(Some(route_filesystem.read_file_as_string(requested)));
        },
        true,
    );

    let read_result = read_document.clone();
    let read_version = read_result.clone();
    let result_recent_files_store = recent_files_store.clone();
    Effect::watch_sync(
        move || {
            read_version
                .try_with(|operation| {
                    operation
                        .as_ref()
                        .and_then(|operation| operation.version().try_get())
                })
                .flatten()
                .unwrap_or_default()
        },
        move |version, _, _| {
            if *version == 0 {
                return;
            }
            let Some(Some(operation)) = read_result.try_get_untracked() else {
                return;
            };
            operation.value().with_untracked(|result| {
                let Some(result) = result else {
                    return;
                };
                match result {
                    Ok(_) => {
                        if let Some(Some(path)) = document_path.try_get_untracked()
                            && let Err(error) = result_recent_files_store.record(&path)
                        {
                            notifications.show_error("Recent files not saved", error.to_string());
                        }
                        let _ = load_error.try_set(None);
                    }
                    Err(error) => {
                        let path = document_path
                            .try_get_untracked()
                            .flatten()
                            .unwrap_or_else(|| PathBuf::from("unknown"));
                        let _ = load_error.try_set(Some(format!(
                            "failed to read Markdown file `{}`: {error}",
                            path.display()
                        )));
                    }
                }
            });
        },
        true,
    );

    let document_read = read_document.clone();
    let document_key_read = read_document;
    let document_key_editor_failure = editor_failure;
    let document = keyed(
        move || {
            let version = document_key_read
                .try_get_untracked()
                .flatten()
                .and_then(|operation| operation.version().try_get_untracked())
                .unwrap_or_default();
            let generation = load_generation.try_get_untracked().unwrap_or_default();
            let path = document_path.try_get_untracked().flatten();
            let load_error = load_error.try_get_untracked().flatten();
            let editor_error = matching_editor_error(&document_key_editor_failure, path.as_deref());
            (generation, version, load_error, editor_error)
        },
        move || {
            let path = document_path.try_get_untracked().flatten();
            let operation = document_read.try_get_untracked().flatten();
            let source = operation
                .as_ref()
                .and_then(|operation| {
                    operation.value().try_with_untracked(|result| {
                        result.as_ref().map(|result| match result {
                            Ok(source) => Ok(source.clone()),
                            Err(error) => Err(error.to_string()),
                        })
                    })
                })
                .flatten();
            let source = load_error
                .try_get_untracked()
                .flatten()
                .map_or(source, |error| Some(Err(error)));
            let editor_error = matching_editor_error(&editor_failure, path.as_deref());
            let loading = operation
                .as_ref()
                .and_then(|operation| operation.pending().try_get_untracked())
                .unwrap_or(false);
            view! {
                <ViewerDocument
                    path=path
                    source=source
                    loading=loading
                    editor_error=editor_error
                />
            }
            .into_view()
        },
    );

    use_viewer_page_styles();

    use_key_event(KeyEventKind::Press, move |key| {
        if key.modifiers != KeyModifiers::NONE {
            return KeyControl::Pass;
        }

        match key.code {
            KeyCode::Char('e') => {
                if let Some(Some(path)) = document_path.try_get_untracked() {
                    editor_session.edit(path, move |path, result| {
                        let failure = result.err().map(|error| {
                            let error = Arc::new(anyhow::Error::new(error));
                            notifications.show_error("Editor failed", error.to_string());
                            EditorFailure { path, error }
                        });
                        if failure.is_none() {
                            notifications
                                .show_success("Editor closed", "Reloaded the Markdown preview.");
                        }
                        editor_failure.set(failure);
                        revision.update(|revision| *revision = revision.wrapping_add(1));
                    });
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
            KeyCode::Char('h') => {
                shortcut_navigate("/", NavigateOptions::default());
                KeyControl::Handled
            }
            KeyCode::Char('b') => {
                shortcut_file_selector
                    .select_with_options(FileSelectorOptions::new().extensions(["md", "markdown"]));
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
                    button_file_selector.select_with_options(
                        FileSelectorOptions::new().extensions(["md", "markdown"]),
                    );
                    AppControl::Continue
                }>"Browse files"</Button>
            </Div>
            <Text class="viewer-page__help">
                "PgUp/Dn scroll | e edit | r reload | h home | b browse | q quit"
            </Text>
        </Div>
    }
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
///
/// # Arguments
///
/// * `editor_failure` — Optional editor failure from this viewer.
/// * `path` — Current canonical or requested document path.
///
/// # Returns
///
/// An optional contextual editor error.
fn matching_editor_error(
    editor_failure: &RwSignal<Option<EditorFailure>>,
    path: Option<&Path>,
) -> Option<String> {
    editor_failure.with_untracked(|failure| {
        failure
            .as_ref()
            .filter(|failure| Some(failure.path.as_path()) == path)
            .map(|failure| failure.error.to_string())
    })
}
