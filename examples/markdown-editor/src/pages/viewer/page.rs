//! Viewer route-level component, local reload state, and shared file synchronization.

use std::path::{Path, PathBuf};

use leptatui::prelude::*;
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

use crate::{
    hooks::{Files, use_files, use_workspace},
    pages::shared::{relative_path, routed_page_style},
    services::{FileSystem, RECENT_FILE_LIMIT, Workspace},
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
/// The route identifies the open document. Only the reload revision belongs
/// to this page instance; recent files and editor handoff use shared file
/// signals.
///
/// # Returns
///
/// A Viewer page component.
#[component]
pub(crate) fn ViewerPage() -> impl IntoView {
    let workspace_context = use_workspace();
    let workspace = workspace_context.workspace;
    let filesystem = workspace_context.filesystem;
    let files = use_files();
    let route_params = use_params_map();
    let revision = RwSignal::new(0_u64);
    let shortcut_navigate = use_navigate();
    let home_navigate = use_navigate();
    let explorer_navigate = use_navigate();
    let document_workspace = workspace.clone();
    let document_files = files.clone();
    let shortcut_workspace = workspace.clone();
    let header_workspace = workspace.clone();
    let edit_request = files.edit_request;
    let editor_failure = files.editor_failure;
    let document = keyed(
        move || {
            (
                route_params
                    .get_untracked()
                    .get("path")
                    .unwrap_or_default()
                    .to_owned(),
                revision.get_untracked(),
            )
        },
        move || {
            let path = synchronize_route(
                &document_workspace,
                filesystem,
                &document_files,
                route_params,
            );
            let editor_error = matching_editor_error(&document_files, path.as_deref());

            view! {
                <ViewerDocument path=path editor_error=editor_error />
            }
        },
    );
    let page_style = routed_page_style();

    use_key_event(KeyEventKind::Press, move |key| {
        if key.modifiers != KeyModifiers::NONE {
            return KeyControl::Pass;
        }

        match key.code {
            KeyCode::Char('e') => {
                if let Some(path) = requested_path(&shortcut_workspace, route_params) {
                    edit_request.set(Some(path));
                    KeyControl::Exit
                } else {
                    KeyControl::Handled
                }
            }
            KeyCode::Char('r') => {
                editor_failure.set(None);
                revision.update(|revision| *revision = revision.wrapping_add(1));
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
                let open_path = requested_path(&header_workspace, route_params)
                    .map_or_else(
                        || String::from("none"),
                        |path| relative_path(header_workspace.root(), &path),
                    );
                view! {
                    <Text class="path-context">{format!("Open: {open_path}")}</Text>
                }
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

/// Returns the workspace path requested by the active Viewer route.
///
/// # Arguments
///
/// * `workspace` — Workspace used to resolve the relative route path.
/// * `params` — Active route parameters.
///
/// # Returns
///
/// An optional requested [`PathBuf`].
fn requested_path(workspace: &Workspace, params: Memo<ParamsMap>) -> Option<PathBuf> {
    params
        .get_untracked()
        .get("path")
        .map(|relative| workspace.root().join(relative))
}

/// Synchronizes the active Viewer route with shared recent-file signals.
///
/// # Arguments
///
/// * `workspace` — Workspace bounding the route path.
/// * `filesystem` — Service used to validate the route path.
/// * `files` — Shared recent-file signals and persistence service.
/// * `params` — Active route parameters.
///
/// # Returns
///
/// An optional canonical or requested path for the document view.
fn synchronize_route(
    workspace: &Workspace,
    filesystem: FileSystem,
    files: &Files,
    params: Memo<ParamsMap>,
) -> Option<PathBuf> {
    let requested = requested_path(workspace, params)?;
    match filesystem.validate_markdown(workspace, &requested) {
        Ok(canonical) => {
            files.recent_files.update(|entries| {
                entries.retain(|entry| entry != &canonical);
                entries.insert(0, canonical.clone());
                entries.truncate(RECENT_FILE_LIMIT);
            });
            files.stored_recent_files.update(|entries| {
                entries.retain(|entry| entry != &canonical);
                entries.insert(0, canonical.clone());
                entries.truncate(RECENT_FILE_LIMIT);
            });
            save_recent_files(files);
            Some(canonical)
        }
        Err(_) => {
            files
                .recent_files
                .update(|entries| entries.retain(|entry| entry != &requested));
            files
                .stored_recent_files
                .update(|entries| entries.retain(|entry| entry != &requested));
            save_recent_files(files);
            Some(requested)
        }
    }
}

/// Persists shared recent-file ordering and records a recoverable error.
///
/// # Arguments
///
/// * `files` — Shared recent-file signals to read and update.
fn save_recent_files(files: &Files) {
    let entries = files.stored_recent_files.get_untracked();
    let error = files
        .recent_files_store
        .save(&entries)
        .err()
        .map(|error| error.to_string());
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
            .map(|failure| failure.message.clone())
    })
}
