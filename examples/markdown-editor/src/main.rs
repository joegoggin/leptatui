//! Standalone Markdown editor application.
//!
//! This binary selects the initial route before constructing the routed
//! component tree and starting the managed terminal.

mod app;
mod cli;
mod contexts;
mod layouts;
mod pages;
mod services;

#[cfg(test)]
mod tests;

use std::{
    io,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context as _, Result};
use clap::Parser;
use leptatui::prelude::*;

use crate::app::{AppRouter, AppRouterProps};
use crate::{cli::Cli, pages::viewer_location};

/// Resolves a file path against the process current directory without requiring it to exist.
///
/// # Arguments
///
/// * `path` — Absolute or current-directory-relative file path.
///
/// # Returns
///
/// An absolute, lexically normalized [`PathBuf`].
///
/// # Errors
///
/// Returns [`io::Error`] if the current directory cannot be read for a relative path.
fn absolute_file_path(path: &Path) -> io::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut normalized = PathBuf::new();

    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if normalized.file_name().is_some() {
                    normalized.pop();
                }
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }

    Ok(normalized)
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
    let cli = Cli::try_parse().context("failed to parse command-line arguments")?;
    let initial_root = match cli.file_path {
        Some(file_path) => viewer_location(&absolute_file_path(&file_path)?),
        None => String::from("/"),
    };
    let view = view! { <AppRouter initial_path=initial_root /> };

    App::new(view)
        .run()
        .await
        .context("Leptatui runtime failed")?;

    Ok(())
}
