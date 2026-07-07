//! App root drawing helpers.
//!
//! This module contains the frame draw wrapper used by the app loop to render
//! an [`AppRoot`] into the active terminal.

use std::io::stdout;

use crossterm::{cursor::SetCursorStyle, execute};

use crate::component::FocusedControl;

use super::{AppRoot, Result, terminal::DefaultTerminal};

/// Draws a root application into the terminal.
///
/// # Arguments
///
/// * `root` — Root application state to render.
/// * `terminal` — Ratatui terminal backend receiving the draw call.
/// # Returns
///
/// An empty [`Result`] on success.
///
/// # Errors
///
/// Returns [`crate::app::Error::Io`] if the terminal draw call fails or root
/// rendering fails through terminal I/O.
pub(super) fn draw_root<R>(root: &mut R, terminal: &mut DefaultTerminal) -> Result<()>
where
    R: AppRoot,
{
    let mut render_result: Result<()> = Ok(());
    let mut focused_control = None;

    terminal.draw(|frame| {
        render_result = root.render(frame);
        focused_control = root.__focused_control();
    })?;

    render_result?;
    execute!(stdout(), cursor_style_for_focused_control(focused_control))?;

    Ok(())
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
        Some(FocusedControl::Button) | None => SetCursorStyle::DefaultUserShape,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
