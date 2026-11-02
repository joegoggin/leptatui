//! Async CRUD-style mock API demo.
//!
//! This binary demonstrates resources, actions, context, stylesheet classes,
//! and the app runner working together for a mock async terminal workflow.
//!
//! # Modules
//!
//! - [`api`] — In-memory asynchronous ticket operations.
//! - [`commands`] — Shared controller state and command dispatch.
//! - [`model`] — Ticket, status, mutation, and result types.
//! - [`ui`] — Reactive components, styles, and render helpers.

#[path = "async_crud/api.rs"]
mod api;
#[path = "async_crud/commands.rs"]
mod commands;
#[path = "async_crud/model.rs"]
mod model;
#[path = "async_crud/ui.rs"]
mod ui;

use leptatui::prelude::*;

use ui::AsyncCrudDemo;

/// Runs the async CRUD example.
///
/// # Returns
///
/// An empty [`Result`] when the app exits successfully.
///
/// # Errors
///
/// Returns [`Error::Io`] if terminal setup, rendering, input, or cleanup fails.
#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    let view = view! { <AsyncCrudDemo /> };
    App::new(view).run().await
}
