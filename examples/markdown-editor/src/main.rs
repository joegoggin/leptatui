//! Standalone Markdown editor application.
//!
//! This binary validates a browsing root before starting Leptatui's managed
//! terminal session, then delegates application behavior to focused domain,
//! infrastructure, controller, and UI modules.

mod cli;
mod controller;
mod domain;
mod editor_process;
mod filesystem;
mod ui;

#[cfg(test)]
mod tests;

use std::error::Error;

use clap::Parser;
use leptatui::prelude::App;

use crate::{
    cli::Cli, controller::Controller, editor_process::EditorProcess, filesystem::FileSystem,
    ui::app_view,
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
/// if terminal setup, rendering, input, or cleanup fails.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let requested_root = Cli::parse().requested_root()?;
    let controller =
        Controller::initialize(&requested_root, FileSystem::new(), EditorProcess::new())?;

    App::new(app_view(controller)).run().await?;
    Ok(())
}
