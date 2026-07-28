//! Standalone Markdown editor application.
//!
//! This binary validates a browsing root before starting Leptatui's managed
//! terminal sessions, then coordinates routed pages and restored-terminal
//! editor sessions while delegating behavior to the application shell, core,
//! service, and page modules.

mod app;
mod core;
mod pages;
mod services;

#[cfg(test)]
mod tests;

use std::{
    cell::{Cell, RefCell},
    error::Error,
    rc::Rc,
};

use clap::Parser;
use leptatui::prelude::App;

use crate::{
    app::app_view_at_path,
    core::{Cli, Controller},
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
    let controller = Rc::new(RefCell::new(Controller::initialize_with_store(
        &requested_root,
        FileSystem::new(),
        EditorProcess::new(),
        RecentFilesStore::standard(),
    )?));
    let mut initial_path = String::from("/");

    loop {
        let edit_requested = Rc::new(Cell::new(false));
        App::new(app_view_at_path(
            Rc::clone(&controller),
            Rc::clone(&edit_requested),
            initial_path.clone(),
        ))
        .run()
        .await?;

        if !edit_requested.get() {
            return Ok(());
        }

        controller.borrow_mut().edit_preview();
        initial_path = {
            let controller = controller.borrow();
            controller.preview().path().map_or_else(
                || String::from("/"),
                |path| viewer_location(controller.workspace().root(), path),
            )
        };
    }
}
