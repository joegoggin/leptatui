//! Standalone Markdown editor application.
//!
//! This binary initializes application services before constructing the routed
//! component tree and starting the managed terminal.

mod app;
mod cli;
mod contexts;
mod hooks;
mod pages;
mod services;

#[cfg(test)]
mod tests;

use std::{ffi::OsString, sync::Arc};

use anyhow::{Context as _, Result};
use clap::Parser;
use leptatui::prelude::*;

use crate::app::{AppRouter, AppRouterProps};
use crate::{
    cli::Cli,
    hooks::{Files, WorkspaceContext},
    services::{EditorProcess, RecentFilesStore, Workspace},
};

/// Parses startup arguments and initializes services needed by the Markdown editor.
///
/// # Arguments
///
/// * `arguments` — Process-style arguments beginning with the binary name.
///
/// # Returns
///
/// A tuple containing the validated workspace, file state, and editor service.
///
/// # Errors
///
/// Returns an [`anyhow::Error`] if CLI parsing, current-directory discovery, or
/// workspace validation fails.
fn initialize<I, T>(arguments: I) -> Result<(WorkspaceContext, Files, EditorProcess)>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::try_parse_from(arguments).context("failed to parse command-line arguments")?;
    let requested_root = cli
        .requested_root()
        .context("failed to determine the browsing root")?;
    let filesystem = use_file_system(&requested_root)
        .context("failed to resolve browsing root")
        .with_context(|| {
            format!(
                "failed to initialize workspace from '{}'",
                requested_root.display()
            )
        })?;
    let recent_files_store = RecentFilesStore::standard();
    let workspace = Workspace::new(filesystem.root().to_path_buf());
    let (recent_paths, stored_paths, recent_error) =
        recent_files_store.load_for_workspace(&filesystem, &workspace);
    let recent_error = recent_error.map(|error| Arc::new(anyhow::Error::new(error)));
    let workspace = WorkspaceContext::new(workspace);
    let files = Files::new(recent_paths, stored_paths, recent_error, recent_files_store);

    Ok((workspace, files, EditorProcess::new()))
}

/// Runs the initialized Markdown editor.
///
/// # Returns
///
/// An empty [`Result`] after a clean application exit.
///
/// # Errors
///
/// Returns an [`anyhow::Error`] if application initialization, terminal setup,
/// rendering, input, or cleanup fails. Editor launch and exit failures remain
/// recoverable preview errors after the TUI resumes.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let (workspace, files, editor_process) = initialize(std::env::args_os())?;
    let view = view! {
        <AppRouter
            workspace=workspace
            files=files
            editor_process=editor_process
        />
    };

    App::new(view)
        .run()
        .await
        .context("Leptatui runtime failed")?;

    Ok(())
}
