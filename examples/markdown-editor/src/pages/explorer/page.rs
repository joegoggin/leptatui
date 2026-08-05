//! Explorer route-level component, local state, and keyboard behavior.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use leptatui::prelude::*;

use crate::{
    contexts::use_notifications,
    pages::viewer_location,
    services::{DirectoryListing, ExplorerEntryKind, volume_root},
};

use super::components::{ExplorerContent, ExplorerContentProps};

/// Renders the standalone current-directory file explorer.
///
/// The listing, selection, and recoverable navigation error belong to this
/// routed page instance. Leaving and returning creates fresh signals starting
/// from the process current directory.
///
/// # Returns
///
/// An Explorer page component or a filesystem initialization error.
#[component]
pub(crate) fn ExplorerPage() -> ViewResult<impl IntoView> {
    let notifications = use_notifications();
    let current_directory = std::fs::canonicalize(std::env::current_dir()?)?;
    let filesystem = use_file_system(volume_root(&current_directory))?;
    let listing = ArcRwSignal::new(DirectoryListing::empty(current_directory.clone()));
    let selection = ArcRwSignal::new(None);
    let error = ArcRwSignal::new(None);
    let requested_directory = ArcRwSignal::new(current_directory.clone());
    let read_directory = ArcRwSignal::new(Some(filesystem.read_dir(&current_directory)));
    let read_directory_result = read_directory.clone();
    let read_directory_version = read_directory.clone();
    let result_requested_directory = requested_directory.clone();
    let result_selection = selection.clone();
    let result_listing = listing.clone();
    let result_error = error.clone();
    Effect::watch_sync(
        move || {
            read_directory_version
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
            let Some(directory) = result_requested_directory.try_get_untracked() else {
                return;
            };
            let Some(Some(operation)) = read_directory_result.try_get_untracked() else {
                return;
            };
            operation.value().with_untracked(|result| {
                let Some(result) = result else {
                    return;
                };
                match result {
                    Ok(entries) => {
                        let next_listing =
                            DirectoryListing::from_file_entries(directory, entries.clone());
                        let _ = result_selection
                            .try_set((!next_listing.entries().is_empty()).then_some(0));
                        let _ = result_listing.try_set(next_listing);
                        let _ = result_error.try_set(None);
                    }
                    Err(source) => {
                        let source = Arc::new(anyhow::Error::new(std::io::Error::new(
                            source.kind(),
                            source.to_string(),
                        )));
                        notifications.show_error("Unable to browse directory", source.to_string());
                        let _ = result_error.try_set(Some(source));
                    }
                }
            });
        },
        true,
    );
    let shortcut_navigate = use_navigate();
    let shortcut_filesystem = filesystem.clone();
    let shortcut_listing = listing.clone();
    let shortcut_selection = selection.clone();
    let shortcut_requested_directory = requested_directory.clone();
    let shortcut_read_directory = read_directory.clone();

    use_key_event(KeyEventKind::Press, move |key| {
        if key.modifiers != KeyModifiers::NONE {
            return KeyControl::Pass;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                select_previous(&shortcut_selection);
                KeyControl::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                select_next(&shortcut_listing, &shortcut_selection);
                KeyControl::Handled
            }
            KeyCode::Enter => {
                if let Some(entry) = selected_entry(&shortcut_listing, &shortcut_selection) {
                    match entry.kind() {
                        ExplorerEntryKind::Directory => {
                            browse(
                                &shortcut_requested_directory,
                                &shortcut_read_directory,
                                &shortcut_filesystem,
                                entry.path(),
                            );
                        }
                        ExplorerEntryKind::Markdown => {
                            let target = viewer_location(entry.path());
                            shortcut_navigate(&target, NavigateOptions::default());
                        }
                    }
                }
                KeyControl::Handled
            }
            KeyCode::Left | KeyCode::Char('h') => {
                browse_parent(
                    &shortcut_listing,
                    &shortcut_requested_directory,
                    &shortcut_read_directory,
                    &shortcut_filesystem,
                );
                KeyControl::Handled
            }
            KeyCode::Esc => {
                shortcut_navigate("/", NavigateOptions::default());
                KeyControl::Handled
            }
            _ => KeyControl::Pass,
        }
    });

    view! {
        <ExplorerContent
            initial_directory=current_directory
            listing=listing.clone()
            selection=selection.clone()
            error=error.clone()
        />
    }
}

/// Returns the currently selected explorer entry.
///
/// # Arguments
///
/// * `listing` — Page-local directory listing signal.
/// * `selection` — Page-local selected index signal.
///
/// # Returns
///
/// An optional cloned explorer entry.
fn selected_entry(
    listing: &ArcRwSignal<DirectoryListing>,
    selection: &ArcRwSignal<Option<usize>>,
) -> Option<crate::services::ExplorerEntry> {
    let selected = selection.get_untracked()?;
    listing.with_untracked(|listing| listing.entries().get(selected).cloned())
}

/// Moves the page-local selection toward the previous entry.
///
/// # Arguments
///
/// * `selection` — Page-local selected index signal.
fn select_previous(selection: &ArcRwSignal<Option<usize>>) {
    selection.update(|selection| {
        if let Some(index) = selection {
            *index = index.saturating_sub(1);
        }
    });
}

/// Moves the page-local selection toward the next entry.
///
/// # Arguments
///
/// * `listing` — Page-local directory listing signal.
/// * `selection` — Page-local selected index signal.
fn select_next(listing: &ArcRwSignal<DirectoryListing>, selection: &ArcRwSignal<Option<usize>>) {
    let last = listing.with_untracked(|listing| listing.entries().len().checked_sub(1));
    selection.update(|selection| {
        if let (Some(index), Some(last)) = (selection, last) {
            *index = index.saturating_add(1).min(last);
        }
    });
}

/// Navigates the page-local explorer to a requested directory.
///
/// Failed navigation records an error without replacing the last valid
/// listing or selection.
///
/// # Arguments
///
/// * `requested` — Signal retaining the latest requested directory.
/// * `read_directory` — Signal retaining the latest directory operation.
/// * `filesystem` — Component-local filesystem handle used for the request.
/// * `requested_directory` — Directory to resolve and list.
///
/// # Returns
///
/// A [`bool`] indicating whether the request was dispatched.
fn browse(
    requested: &ArcRwSignal<PathBuf>,
    read_directory: &ArcRwSignal<Option<FileOperation<Vec<FileEntry>>>>,
    filesystem: &FileSystem,
    requested_directory: &Path,
) -> bool {
    let _ = requested.try_set(requested_directory.to_path_buf());
    let _ = read_directory.try_set(Some(filesystem.read_dir(requested_directory)));
    true
}

/// Navigates the page-local explorer to its parent directory.
///
/// # Arguments
///
/// * `listing` — Page-local directory listing signal.
/// * `requested` — Signal retaining the latest requested directory.
/// * `read_directory` — Signal retaining the latest directory operation.
/// * `filesystem` — Component-local filesystem handle used for the request.
///
/// # Returns
///
/// A [`bool`] indicating whether the explorer moved to its parent.
fn browse_parent(
    listing: &ArcRwSignal<DirectoryListing>,
    requested: &ArcRwSignal<PathBuf>,
    read_directory: &ArcRwSignal<Option<FileOperation<Vec<FileEntry>>>>,
    filesystem: &FileSystem,
) -> bool {
    let directory: PathBuf = listing.with_untracked(|listing| listing.directory().to_path_buf());
    if directory == filesystem.root() {
        return false;
    }
    let Some(parent) = directory.parent() else {
        return false;
    };

    browse(requested, read_directory, filesystem, parent)
}
