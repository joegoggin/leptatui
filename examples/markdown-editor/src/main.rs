//! Standalone Markdown editor application.
//!
//! This binary validates a browsing root before starting Leptatui's managed
//! terminal sessions, then coordinates routed pages and restored-terminal
//! editor sessions while delegating behavior to the application shell, hooks,
//! services, and page modules.

mod app;
mod cli;
mod hooks;
mod pages;
mod services;

#[cfg(test)]
mod tests;

use std::error::Error;

use clap::Parser;
use leptatui::prelude::{App, GetUntracked, Owner, Set};

use crate::{
    app::app_view_at_path,
    cli::Cli,
    hooks::{EditorFailure, Files, WorkspaceContext},
    pages::viewer_location,
    services::{EditorProcess, FileSystem, RecentFilesStore},
};

/// Validates startup configuration and runs the Markdown editor.
///
/// # Returns
///
/// An empty [`Result`] after a clean application exit.
///
/// # Errors
///
/// Returns an error if the browsing root cannot be resolved to a directory or
/// if terminal setup, rendering, input, or cleanup fails. Editor launch and
/// exit failures become recoverable preview errors after the TUI restarts.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let requested_root = Cli::parse().requested_root()?;
    let owner = Owner::new();
    let filesystem = FileSystem::new();
    let editor_process = EditorProcess::new();
    let recent_files_store = RecentFilesStore::standard();
    let workspace = filesystem.validate_root(&requested_root)?;
    let (recent_paths, stored_paths, recent_error) =
        recent_files_store.load_for_workspace(filesystem, &workspace);
    let workspace_context = WorkspaceContext::new(workspace, filesystem);
    let files =
        owner.with(|| Files::new(recent_paths, stored_paths, recent_error, recent_files_store));
    let mut initial_path = String::from("/");

    loop {
        files.edit_request.set(None);
        let view = owner.with(|| {
            app_view_at_path(
                workspace_context.clone(),
                files.clone(),
                initial_path.clone(),
            )
        });
        App::new(view).run().await?;

        let Some(path) = files.edit_request.get_untracked() else {
            return Ok(());
        };

        match editor_process.edit(&path) {
            Ok(()) => files.editor_failure.set(None),
            Err(error) => files.editor_failure.set(Some(EditorFailure {
                path: path.clone(),
                message: error.to_string(),
            })),
        }
        initial_path = viewer_location(workspace_context.root(), &path);
    }
}
