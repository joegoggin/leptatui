//! Explorer route-level component, local state, and keyboard behavior.

use std::path::{Path, PathBuf};

use leptatui::prelude::*;

use crate::{
    pages::viewer_location,
    services::{DirectoryListing, ExplorerEntryKind, FileSystem, Workspace},
};

use super::components::{ExplorerContent, ExplorerContentProps};

/// Renders the standalone workspace file explorer.
///
/// The listing, selection, and recoverable navigation error belong to this
/// routed page instance. Leaving and returning to the route creates fresh
/// signals rooted at the workspace.
///
/// # Returns
///
/// An Explorer page component.
#[component]
pub(crate) fn ExplorerPage() -> impl IntoView {
    let workspace = expect_context::<Workspace>();
    let filesystem = expect_context::<FileSystem>();
    let root = workspace.root().to_path_buf();
    let (initial_listing, initial_error) = match filesystem.list_directory(&workspace, &root) {
        Ok(listing) => (listing, None),
        Err(error) => (DirectoryListing::empty(root), Some(error.to_string())),
    };
    let initial_selection = (!initial_listing.entries().is_empty()).then_some(0);
    let listing = RwSignal::new(initial_listing);
    let selection = RwSignal::new(initial_selection);
    let error = RwSignal::new(initial_error);
    let shortcut_workspace = workspace.clone();
    let content_workspace = workspace.clone();
    let shortcut_navigate = use_navigate();

    use_key_event(KeyEventKind::Press, move |key| {
        if key.modifiers != KeyModifiers::NONE {
            return KeyControl::Pass;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                select_previous(selection);
                KeyControl::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                select_next(listing, selection);
                KeyControl::Handled
            }
            KeyCode::Enter => {
                if let Some(entry) = selected_entry(listing, selection) {
                    match entry.kind() {
                        ExplorerEntryKind::Directory => {
                            browse(
                                &shortcut_workspace,
                                listing,
                                selection,
                                error,
                                filesystem,
                                entry.path(),
                            );
                        }
                        ExplorerEntryKind::Markdown => {
                            let target = viewer_location(shortcut_workspace.root(), entry.path());
                            shortcut_navigate(&target, NavigateOptions::default());
                        }
                    }
                }
                KeyControl::Handled
            }
            KeyCode::Left | KeyCode::Char('h') => {
                browse_parent(&shortcut_workspace, listing, selection, error, filesystem);
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
            workspace=content_workspace
            listing=listing
            selection=selection
            error=error
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
    listing: RwSignal<DirectoryListing>,
    selection: RwSignal<Option<usize>>,
) -> Option<crate::services::ExplorerEntry> {
    let selected = selection.get_untracked()?;
    listing.with_untracked(|listing| listing.entries().get(selected).cloned())
}

/// Moves the page-local selection toward the previous entry.
///
/// # Arguments
///
/// * `selection` — Page-local selected index signal.
fn select_previous(selection: RwSignal<Option<usize>>) {
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
fn select_next(listing: RwSignal<DirectoryListing>, selection: RwSignal<Option<usize>>) {
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
/// * `workspace` — Workspace bounding navigation.
/// * `listing` — Page-local directory listing signal.
/// * `selection` — Page-local selected index signal.
/// * `error` — Page-local recoverable error signal.
/// * `filesystem` — Service used to discover the requested directory.
/// * `requested_directory` — Directory to resolve and list.
///
/// # Returns
///
/// A [`bool`] indicating whether navigation succeeded.
fn browse(
    workspace: &Workspace,
    listing: RwSignal<DirectoryListing>,
    selection: RwSignal<Option<usize>>,
    error: RwSignal<Option<String>>,
    filesystem: FileSystem,
    requested_directory: &Path,
) -> bool {
    match filesystem.list_directory(workspace, requested_directory) {
        Ok(next_listing) => {
            selection.set((!next_listing.entries().is_empty()).then_some(0));
            listing.set(next_listing);
            error.set(None);
            true
        }
        Err(source) => {
            error.set(Some(source.to_string()));
            false
        }
    }
}

/// Navigates the page-local explorer to its parent directory.
///
/// # Arguments
///
/// * `workspace` — Workspace bounding parent navigation.
/// * `listing` — Page-local directory listing signal.
/// * `selection` — Page-local selected index signal.
/// * `error` — Page-local recoverable error signal.
/// * `filesystem` — Service used to discover the parent directory.
///
/// # Returns
///
/// A [`bool`] indicating whether the explorer moved to its parent.
fn browse_parent(
    workspace: &Workspace,
    listing: RwSignal<DirectoryListing>,
    selection: RwSignal<Option<usize>>,
    error: RwSignal<Option<String>>,
    filesystem: FileSystem,
) -> bool {
    let directory: PathBuf = listing.with_untracked(|listing| listing.directory().to_path_buf());
    if directory == workspace.root() {
        return false;
    }
    let Some(parent) = directory.parent() else {
        return false;
    };

    browse(workspace, listing, selection, error, filesystem, parent)
}
