//! Viewer route-level component, local reload state, and shared file synchronization.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use leptatui::prelude::*;
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

use crate::{
    contexts::{NotificationContext, use_notifications},
    hooks::{Files, use_files, use_workspace},
    pages::shared::{relative_path, routed_page_style},
    services::{EditorSession, RECENT_FILE_LIMIT, is_markdown_path},
};

use super::components::viewer_document;

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
/// The route identifies the open document. Only the reload revision belongs
/// to this page instance; recent files and editor failures use shared signals.
///
/// # Returns
///
/// A Viewer page component or a filesystem initialization error.
#[component]
pub(crate) fn ViewerPage() -> ViewResult<impl IntoView> {
    let workspace_context = use_workspace();
    let notifications = use_notifications();
    let workspace = workspace_context.workspace;
    let filesystem = use_file_system(workspace.root())?;
    let files = use_files();
    let editor_session = expect_context::<EditorSession>();
    let route_params = use_params_map();
    let revision = RwSignal::new(0_u64);
    let shortcut_navigate = use_navigate();
    let home_navigate = use_navigate();
    let explorer_navigate = use_navigate();
    let open_path = route_params
        .get_untracked()
        .get("path")
        .map(|relative| workspace.root().join(relative))
        .map_or_else(
            || String::from("none"),
            |path| relative_path(workspace.root(), &path),
        );
    let editor_failure = files.editor_failure;
    let document_path = RwSignal::new(None::<PathBuf>);
    let load_error = RwSignal::new(None::<String>);
    let load_generation = RwSignal::new(0_u64);
    let read_document = RwSignal::new(None::<FileOperation<String>>);
    let read_for_route = read_document;
    let route_filesystem = filesystem.clone();
    let route_workspace = workspace.clone();
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
        move |(relative, _), _, _| {
            let _ = load_generation.try_update(|generation| {
                *generation = generation.wrapping_add(1);
            });
            let _ = document_path.try_set(None);
            let _ = load_error.try_set(None);
            let _ = read_for_route.try_set(None);
            if relative.is_empty() {
                return;
            }
            let requested = route_workspace.root().join(relative);
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

    let read_result = read_document;
    let read_version = read_result;
    let recent_files = files.clone();
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
                        if let Some(Some(path)) = document_path.try_get_untracked() {
                            record_recent_file(&recent_files, path, notifications);
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

    let document_files = files.clone();
    let document_read = read_document;
    let document_key_read = read_document;
    let document_key_files = files.clone();
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
            let editor_error = matching_editor_error(&document_key_files, path.as_deref());
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
            let editor_error = matching_editor_error(&document_files, path.as_deref());
            let loading = operation
                .as_ref()
                .and_then(|operation| operation.pending().try_get_untracked())
                .unwrap_or(false);
            viewer_document(path, source, loading, editor_error)
        },
    );
    let page_style = routed_page_style();

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
                            crate::hooks::EditorFailure { path, error }
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
                shortcut_navigate("/files", NavigateOptions::default());
                KeyControl::Handled
            }
            _ => KeyControl::Pass,
        }
    });

    view! {
        <Div class="page" style=page_style>
            <Text class="page-title">"Markdown viewer"</Text>
            <Text class="path-context">{format!("Open: {open_path}")}</Text>
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

/// Promotes one successfully loaded path through shared recent-file signals.
///
/// # Arguments
///
/// * `files` — Shared recent-file signals and persistence service.
/// * `canonical` — Successfully loaded canonical Markdown path.
/// * `notifications` — Shared notification state for persistence failures.
fn record_recent_file(files: &Files, canonical: PathBuf, notifications: NotificationContext) {
    files.recent_files.update(|entries| {
        entries.retain(|entry| entry != &canonical);
        entries.insert(0, canonical.clone());
        entries.truncate(RECENT_FILE_LIMIT);
    });
    files.stored_recent_files.update(|entries| {
        entries.retain(|entry| entry != &canonical);
        entries.insert(0, canonical);
        entries.truncate(RECENT_FILE_LIMIT);
    });
    save_recent_files(files, notifications);
}

/// Persists shared recent-file ordering and records a recoverable error.
///
/// # Arguments
///
/// * `files` — Shared recent-file signals to read and update.
/// * `notifications` — Shared notification state for save failures.
fn save_recent_files(files: &Files, notifications: NotificationContext) {
    let entries = files.stored_recent_files.get_untracked();
    let error = files
        .recent_files_store
        .save(&entries)
        .err()
        .map(|error| Arc::new(anyhow::Error::new(error)));
    if let Some(error) = &error {
        notifications.show_error("Recent files not saved", error.to_string());
    }
    files.recent_files_error.set(error);
}

/// Returns an editor diagnostic only when it belongs to the open path.
///
/// # Arguments
///
/// * `files` — Shared file signals containing an optional editor failure.
/// * `path` — Current canonical or requested document path.
///
/// # Returns
///
/// An optional contextual editor error.
fn matching_editor_error(files: &Files, path: Option<&Path>) -> Option<String> {
    files.editor_failure.with_untracked(|failure| {
        failure
            .as_ref()
            .filter(|failure| Some(failure.path.as_path()) == path)
            .map(|failure| failure.error.to_string())
    })
}
