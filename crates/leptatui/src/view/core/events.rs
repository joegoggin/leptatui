//! Default event traversal shared by terminal view trees.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};

use crate::{
    app::{AppControl, Result},
    component::KeyControl,
};

use super::contract::View;

pub(crate) fn handle_view_event<V>(view: &mut V, event: Event) -> Result<AppControl>
where
    V: View + ?Sized,
{
    if let Event::Key(key) = event {
        return Ok(handle_view_key_event(view, key)?.into());
    }

    view.__dispatch_event(&event)
}

/// Dispatches custom and built-in key behavior through a view tree.
pub(crate) fn handle_view_key_event<V>(view: &mut V, key: KeyEvent) -> Result<KeyControl>
where
    V: View + ?Sized,
{
    let control = view.__dispatch_key_event(key)?;
    if control == KeyControl::Pass {
        return handle_default_view_key_event(view, key);
    }

    Ok(control)
}

/// Handles built-in scrolling, focus, editing, and activation keys.
pub(crate) fn handle_default_view_key_event<V>(view: &mut V, key: KeyEvent) -> Result<KeyControl>
where
    V: View + ?Sized,
{
    if key.kind != KeyEventKind::Press {
        return Ok(KeyControl::Pass);
    }

    if let Some(control) = view.__handle_form_key(key) {
        clear_scroll_to_top_key_pending(view);
        return Ok(control);
    }

    if let Some(control) = view.__handle_focused_input_key(key) {
        clear_scroll_to_top_key_pending(view);
        return Ok(control);
    }

    let control = match key.code {
        KeyCode::Down | KeyCode::Char('j') => handle_scroll_key(view, 1),
        KeyCode::Up | KeyCode::Char('k') => handle_scroll_key(view, -1),
        KeyCode::PageDown => handle_scroll_key(view, 5),
        KeyCode::PageUp => handle_scroll_key(view, -5),
        KeyCode::Char('g') => handle_scroll_to_top_key(view),
        KeyCode::Char('G') => {
            clear_scroll_to_top_key_pending(view);
            key_control_from_bool(view.__scroll_first_overflowing_to_bottom())
        }
        KeyCode::Tab | KeyCode::BackTab => {
            clear_scroll_to_top_key_pending(view);
            let count = view.__focusable_count();
            if count == 0 {
                KeyControl::Pass
            } else {
                let direction = if key.code == KeyCode::Tab {
                    FocusDirection::Forward
                } else {
                    FocusDirection::Backward
                };
                move_focus(view, direction, count);
                KeyControl::Handled
            }
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            clear_scroll_to_top_key_pending(view);
            view.__activate_focused_button()
                .map_or(KeyControl::Pass, KeyControl::from)
        }
        _ => {
            clear_scroll_to_top_key_pending(view);
            KeyControl::Pass
        }
    };

    Ok(control)
}

/// Moves focus by one position in the requested direction.
fn move_focus<V>(view: &mut V, direction: FocusDirection, count: usize)
where
    V: View + ?Sized,
{
    let mut index = 0;
    let focused = view.__focused_index_inner(&mut index);
    let target = match (focused, direction) {
        (Some(index), FocusDirection::Forward) => (index + 1) % count,
        (Some(0), FocusDirection::Backward) => count - 1,
        (Some(index), FocusDirection::Backward) => index - 1,
        (None, FocusDirection::Forward) => 0,
        (None, FocusDirection::Backward) => count - 1,
    };
    let mut index = 0;
    view.__set_focus_by_index_inner(target, &mut index);
}

/// Handles a relative scroll key.
fn handle_scroll_key<V>(view: &mut V, delta: i16) -> KeyControl
where
    V: View + ?Sized,
{
    clear_scroll_to_top_key_pending(view);
    key_control_from_bool(view.__scroll_first_overflowing(delta))
}

/// Handles the two-key `gg` scroll-to-top sequence.
fn handle_scroll_to_top_key<V>(view: &mut V) -> KeyControl
where
    V: View + ?Sized,
{
    if take_scroll_to_top_key_pending(view) {
        key_control_from_bool(view.__scroll_first_overflowing_to_top())
    } else if view.__has_overflowing_scroll_target() {
        set_scroll_to_top_key_pending(view, true);
        KeyControl::Handled
    } else {
        KeyControl::Pass
    }
}

/// Stores whether the first `g` in `gg` has been pressed.
fn set_scroll_to_top_key_pending<V>(view: &V, pending: bool)
where
    V: View + ?Sized,
{
    view.__set_scroll_to_top_key_pending(pending);
}

/// Clears and returns whether the first `g` in `gg` was pressed.
fn take_scroll_to_top_key_pending<V>(view: &V) -> bool
where
    V: View + ?Sized,
{
    view.__take_scroll_to_top_key_pending()
}

/// Clears any pending first `g` key.
fn clear_scroll_to_top_key_pending<V>(view: &V)
where
    V: View + ?Sized,
{
    set_scroll_to_top_key_pending(view, false);
}
/// Direction used to move focus through focusable controls.
#[derive(Clone, Copy)]
enum FocusDirection {
    /// Move focus to the next focusable control.
    Forward,
    /// Move focus to the previous focusable control.
    Backward,
}

/// Converts a handled flag into the matching key traversal control.
fn key_control_from_bool(handled: bool) -> KeyControl {
    if handled {
        KeyControl::Handled
    } else {
        KeyControl::Pass
    }
}
