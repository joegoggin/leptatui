//! App root drawing helpers.
//!
//! This module contains the frame draw wrapper used by the app loop to render
//! an [`AppRoot`] into the active terminal.

use crate::style::Stylesheet;

use super::{AppRoot, Result, terminal::DefaultTerminal};

/// Draws a root application into the terminal.
///
/// # Arguments
///
/// * `root` — Root application state to render.
/// * `terminal` — Ratatui terminal backend receiving the draw call.
/// * `stylesheet` — Application stylesheet for resolving node styles.
///
/// # Returns
///
/// An empty [`Result`] on success.
///
/// # Errors
///
/// Returns [`crate::app::Error::Io`] if the terminal draw call fails or root
/// rendering fails through terminal I/O.
pub(super) fn draw_root<R>(
    root: &mut R,
    terminal: &mut DefaultTerminal,
    stylesheet: &Stylesheet,
) -> Result<()>
where
    R: AppRoot,
{
    let mut render_result: Result<()> = Ok(());

    terminal.draw(|frame| {
        render_result = root.render(frame, stylesheet);
    })?;

    render_result
}
