//! Standalone Markdown editor application.
//!
//! This binary constructs the prop-free application root and delegates
//! initialization, state, routing, and restored-terminal editor sessions to
//! components.

mod app;
mod cli;
mod hooks;
mod pages;
mod services;

#[cfg(test)]
mod tests;

use leptatui::prelude::*;

use crate::app::AppRouter;

/// Runs the component-owned Markdown editor.
///
/// # Returns
///
/// An empty [`Result`] after a clean application exit.
///
/// # Errors
///
/// Returns an error if component initialization, terminal setup, rendering,
/// input, or cleanup fails. Editor launch and exit failures remain recoverable
/// preview errors after the TUI resumes.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let view = view! { <AppRouter /> };

    App::new(view).run().await?;

    Ok(())
}
