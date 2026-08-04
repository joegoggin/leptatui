//! App root drawing helpers.
//!
//! This module contains the frame draw wrapper used by the app loop to render
//! an [`AppRoot`] into the active terminal.

use std::io::stdout;

use crossterm::{cursor::SetCursorStyle, execute};

use crate::{
    View,
    component::{FocusedControl, RenderCtx},
    context,
    terminal_image::TerminalImageSupport,
    view::{ComponentView, core::layout::render_with_layout},
};

use super::{AppHandle, AppRoot, ErrorScreenRegistry, Result, terminal::DefaultTerminal};

/// Draws a root application into the terminal.
///
/// # Arguments
///
/// * `root` — Root application state to render.
/// * `terminal` — Ratatui terminal backend receiving the draw call.
/// * `terminal_images` — Terminal image support detected for the session.
/// * `app_handle` — Runtime handle provided to managed components.
/// * `error_screens` — Runner registry for standalone error screens.
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
    terminal_images: &TerminalImageSupport,
    app_handle: &AppHandle,
    error_screens: &ErrorScreenRegistry,
) -> Result<()>
where
    R: AppRoot,
{
    let mut render_result: Result<()> = Ok(());
    let mut focused_control = None;

    terminal.draw(|frame| {
        context::hooks::__with_context_scope(|| {
            context::provide_context(terminal_images.clone());
            context::provide_context(app_handle.clone());
            context::provide_context(error_screens.clone());

            if let Some(screen) = error_screens.active() {
                render_result = render_error_screen(&screen, frame);
                focused_control = screen.focused_control();
                return;
            }

            render_result = root.render(frame);
            if render_result.is_ok()
                && let Some(screen) = error_screens.active()
            {
                render_result = render_error_screen(&screen, frame);
                focused_control = screen.focused_control();
            } else {
                focused_control = root.__focused_control();
            }
        });
    })?;

    render_result?;
    execute!(stdout(), cursor_style_for_focused_control(focused_control))?;

    Ok(())
}

/// Renders one registered error screen as the complete frame root.
///
/// # Arguments
///
/// * `screen` — Active error-screen component boundary.
/// * `frame` — Ratatui frame receiving the standalone screen.
///
/// # Returns
///
/// An empty [`Result`] after the screen renders successfully.
///
/// # Errors
///
/// Returns [`crate::Error::Io`] if error-screen rendering performs terminal
/// I/O that fails.
fn render_error_screen(screen: &ComponentView, frame: &mut ratatui::Frame<'_>) -> Result<()> {
    let mut ctx = RenderCtx::new(frame);
    render_with_layout(screen, &mut ctx, |ctx| View::render(screen, ctx))
}

/// Returns the terminal cursor style for the focused built-in control.
fn cursor_style_for_focused_control(focused_control: Option<FocusedControl>) -> SetCursorStyle {
    match focused_control {
        Some(FocusedControl::Input {
            insert_mode: true, ..
        })
        | Some(FocusedControl::TextArea {
            insert_mode: true, ..
        }) => SetCursorStyle::BlinkingBar,
        Some(FocusedControl::Input {
            insert_mode: false, ..
        })
        | Some(FocusedControl::TextArea {
            insert_mode: false, ..
        }) => SetCursorStyle::BlinkingBlock,
        Some(FocusedControl::Button | FocusedControl::Link) | None => {
            SetCursorStyle::DefaultUserShape
        }
    }
}

#[cfg(test)]
/// Unit tests for app root cursor rendering helpers.
mod tests {
    use super::*;

    /// Verifies insert-mode editable controls request a blinking bar cursor.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// FocusedControl::Input { insert_mode: true }
    /// FocusedControl::TextArea { insert_mode: true }
    /// ```
    ///
    /// # Assertions
    ///
    /// - Focused inputs in insert mode map to `SetCursorStyle::BlinkingBar`.
    /// - Focused text areas in insert mode map to `SetCursorStyle::BlinkingBar`.
    #[test]
    fn insert_mode_editable_controls_use_blinking_bar_cursor() {
        assert_eq!(
            cursor_style_for_focused_control(Some(FocusedControl::Input {
                insert_mode: true,
                visual_mode: false,
            })),
            SetCursorStyle::BlinkingBar
        );
        assert_eq!(
            cursor_style_for_focused_control(Some(FocusedControl::TextArea {
                insert_mode: true,
                visual_mode: false,
            })),
            SetCursorStyle::BlinkingBar
        );
    }

    /// Verifies normal and visual editable controls request a blinking block cursor.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// FocusedControl::Input { insert_mode: false }
    /// FocusedControl::TextArea { insert_mode: false }
    /// ```
    ///
    /// # Assertions
    ///
    /// - Focused inputs outside insert mode map to `SetCursorStyle::BlinkingBlock`.
    /// - Focused text areas outside insert mode map to `SetCursorStyle::BlinkingBlock`.
    /// - Visual mode does not change the cursor shape.
    #[test]
    fn normal_mode_editable_controls_use_blinking_block_cursor() {
        assert_eq!(
            cursor_style_for_focused_control(Some(FocusedControl::Input {
                insert_mode: false,
                visual_mode: false,
            })),
            SetCursorStyle::BlinkingBlock
        );
        assert_eq!(
            cursor_style_for_focused_control(Some(FocusedControl::Input {
                insert_mode: false,
                visual_mode: true,
            })),
            SetCursorStyle::BlinkingBlock
        );
        assert_eq!(
            cursor_style_for_focused_control(Some(FocusedControl::TextArea {
                insert_mode: false,
                visual_mode: false,
            })),
            SetCursorStyle::BlinkingBlock
        );
        assert_eq!(
            cursor_style_for_focused_control(Some(FocusedControl::TextArea {
                insert_mode: false,
                visual_mode: true,
            })),
            SetCursorStyle::BlinkingBlock
        );
    }

    /// Verifies non-editable focus keeps the user's default cursor shape.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// FocusedControl::Button
    /// None
    /// ```
    ///
    /// # Assertions
    ///
    /// - Focused buttons map to `SetCursorStyle::DefaultUserShape`.
    /// - Missing focused controls map to `SetCursorStyle::DefaultUserShape`.
    #[test]
    fn non_editable_focus_uses_default_user_cursor_shape() {
        assert_eq!(
            cursor_style_for_focused_control(Some(FocusedControl::Button)),
            SetCursorStyle::DefaultUserShape
        );
        assert_eq!(
            cursor_style_for_focused_control(None),
            SetCursorStyle::DefaultUserShape
        );
    }
}
