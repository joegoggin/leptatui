//! Standalone Markdown editor application.
//!
//! This binary validates a browsing root before starting Leptatui's managed
//! terminal sessions, then coordinates restored-terminal editor sessions while
//! delegating application behavior to focused domain, infrastructure,
//! controller, and UI modules.

mod cli;
mod controller;
mod domain;
mod editor_process;
mod filesystem;
mod ui;

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
/// if terminal setup, rendering, input, or cleanup fails. Editor launch and
/// exit failures become recoverable preview errors after the TUI restarts.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let requested_root = Cli::parse().requested_root()?;
    let controller = Rc::new(RefCell::new(Controller::initialize(
        &requested_root,
        FileSystem::new(),
        EditorProcess::new(),
    )?));

    loop {
        let edit_requested = Rc::new(Cell::new(false));
        App::new(app_view(Rc::clone(&controller), Rc::clone(&edit_requested)))
            .run()
            .await?;

        if !edit_requested.get() {
            return Ok(());
        }

        controller.borrow_mut().edit_preview();
    }
}
