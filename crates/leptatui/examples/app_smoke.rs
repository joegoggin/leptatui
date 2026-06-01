//! Minimal app runner smoke example.
//!
//! This binary renders a small static node tree and exits when the user presses
//! `q` or `Esc`.

use crossterm::event::{Event, KeyCode, KeyEventKind};
use leptatui::prelude::*;

/// Root component for the smoke example.
struct Root;

impl Component for Root {
    /// Renders the smoke example UI.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context for the current frame.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if node rendering performs terminal I/O that fails.
    fn render(&mut self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        ctx.render_node(&block(column([
            text("Leptatui smoke runner. Press q or Esc to quit."),
            button("Quit"),
        ])))
    }

    /// Handles quit keys for the smoke example.
    ///
    /// # Arguments
    ///
    /// * `event` — Terminal event emitted by Crossterm.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value indicating whether to continue or exit.
    fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        if matches!(
            event,
            Event::Key(key)
                if key.kind == KeyEventKind::Press
                    && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        ) {
            return Ok(AppControl::Exit);
        }

        Ok(AppControl::Continue)
    }
}

/// Runs the smoke example application.
///
/// # Returns
///
/// An empty [`Result`] when the app exits successfully.
///
/// # Errors
///
/// Returns [`Error::Io`] if terminal setup, rendering, input, or cleanup fails.
/// Returns [`Error::EventTask`] if the blocking event task fails.
#[tokio::main]
async fn main() -> Result<()> {
    App::new(Root).run().await
}
