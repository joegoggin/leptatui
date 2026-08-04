//! Default event traversal shared by terminal view trees.

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use crate::{
    Axes,
    app::{AppControl, EventOutcome, LayoutMode, Result},
    component::KeyControl,
};

use super::contract::View;

pub(crate) fn handle_view_event<V>(view: &mut V, event: Event) -> Result<AppControl>
where
    V: View + ?Sized,
{
    handle_view_event_with_layout(view, event).map(|outcome| outcome.control)
}

/// Dispatches one event and reports whether the next frame may reuse layout.
///
/// # Arguments
///
/// * `view` — View tree receiving the event.
/// * `event` — Crossterm event to dispatch.
///
/// # Returns
///
/// An [`EventOutcome`] containing application control and layout requirements.
///
/// # Errors
///
/// Returns [`crate::Error`] if custom handling or activation fails.
pub(crate) fn handle_view_event_with_layout<V>(view: &mut V, event: Event) -> Result<EventOutcome>
where
    V: View + ?Sized,
{
    if let Event::Key(key) = event {
        let outcome = handle_view_key_event_with_layout(view, key)?;
        return Ok(EventOutcome {
            control: outcome.control.into(),
            layout: outcome.layout,
        });
    }
    if let Event::Mouse(mouse) = event {
        let control = view.__dispatch_event(&event)?;
        if control == AppControl::Exit {
            return Ok(EventOutcome::recompute(control));
        }
        return handle_default_view_mouse_event_with_layout(view, mouse);
    }

    view.__dispatch_event(&event).map(EventOutcome::recompute)
}

/// Handles built-in mouse focus, activation, and positioned scrolling.
///
/// # Arguments
///
/// * `view` — View tree receiving built-in mouse behavior.
/// * `mouse` — Crossterm mouse event to handle.
///
/// # Returns
///
/// A [`Result`] containing the [`AppControl`] produced by mouse handling.
///
/// # Errors
///
/// Returns [`crate::Error::LinkOpen`] if clicking a focused link cannot open
/// its target.
pub(crate) fn handle_default_view_mouse_event<V>(
    view: &mut V,
    mouse: MouseEvent,
) -> Result<AppControl>
where
    V: View + ?Sized,
{
    handle_default_view_mouse_event_with_layout(view, mouse).map(|outcome| outcome.control)
}

/// Handles built-in mouse behavior and reports scroll-only repaint eligibility.
///
/// # Arguments
///
/// * `view` — View tree receiving built-in mouse behavior.
/// * `mouse` — Mouse event to handle.
///
/// # Returns
///
/// An [`EventOutcome`] containing application control and layout requirements.
///
/// # Errors
///
/// Returns [`crate::Error`] if activating a focused control fails.
fn handle_default_view_mouse_event_with_layout<V>(
    view: &mut V,
    mouse: MouseEvent,
) -> Result<EventOutcome>
where
    V: View + ?Sized,
{
    match mouse.kind {
        MouseEventKind::Moved => {
            view.__focus_control_at_position(mouse.column, mouse.row);
            Ok(EventOutcome::recompute(AppControl::Continue))
        }
        MouseEventKind::Down(MouseButton::Left) => {
            if view.__focus_control_at_position(mouse.column, mouse.row) {
                return view.__activate_focused_button().map(|control| {
                    EventOutcome::recompute(control.unwrap_or(AppControl::Continue))
                });
            }
            Ok(EventOutcome::recompute(AppControl::Continue))
        }
        MouseEventKind::Up(MouseButton::Left) => {
            view.__focus_control_at_position(mouse.column, mouse.row);
            Ok(EventOutcome::recompute(AppControl::Continue))
        }
        MouseEventKind::ScrollDown => Ok(scroll_mouse_outcome(scroll_at_position(
            view,
            mouse.column,
            mouse.row,
            Axes::new(0, 1),
        ))),
        MouseEventKind::ScrollUp => Ok(scroll_mouse_outcome(scroll_at_position(
            view,
            mouse.column,
            mouse.row,
            Axes::new(0, -1),
        ))),
        MouseEventKind::ScrollLeft => Ok(scroll_mouse_outcome(scroll_at_position(
            view,
            mouse.column,
            mouse.row,
            Axes::new(-1, 0),
        ))),
        MouseEventKind::ScrollRight => Ok(scroll_mouse_outcome(scroll_at_position(
            view,
            mouse.column,
            mouse.row,
            Axes::new(1, 0),
        ))),
        MouseEventKind::Down(_) | MouseEventKind::Up(_) | MouseEventKind::Drag(_) => {
            Ok(EventOutcome::recompute(AppControl::Continue))
        }
    }
}

/// Converts a mouse-scroll result into its next-frame layout requirement.
///
/// # Arguments
///
/// * `scrolled` — Whether an overflowing container changed offset.
///
/// # Returns
///
/// An [`EventOutcome`] permitting reuse only after successful scrolling.
fn scroll_mouse_outcome(scrolled: bool) -> EventOutcome {
    if scrolled {
        EventOutcome::reuse(AppControl::Continue)
    } else {
        EventOutcome::recompute(AppControl::Continue)
    }
}

/// Scrolls the innermost overflow container at a pointer position.
///
/// Falls back to the first matching container when no rendered hit area
/// consumes the event.
///
/// # Arguments
///
/// * `view` — View tree receiving the scroll event.
/// * `column` — Zero-based terminal column under the pointer.
/// * `row` — Zero-based terminal row under the pointer.
/// * `delta` — Signed horizontal and vertical cell deltas.
///
/// # Returns
///
/// A [`bool`] indicating whether an overflowing container changed offset.
fn scroll_at_position<V>(view: &mut V, column: u16, row: u16, delta: Axes<i16>) -> bool
where
    V: View + ?Sized,
{
    if view.__scroll_overflowing_at_position(column, row, delta) {
        true
    } else {
        view.__scroll_first_overflowing(delta)
    }
}

/// Key propagation result paired with the next frame's layout requirement.
struct KeyEventOutcome {
    /// Resulting key propagation control.
    control: KeyControl,
    /// Required work before painting the next frame.
    layout: LayoutMode,
}

impl KeyEventOutcome {
    /// Creates an outcome that conservatively recomputes layout.
    ///
    /// # Arguments
    ///
    /// * `control` — Key propagation control emitted by built-in handling.
    ///
    /// # Returns
    ///
    /// A [`KeyEventOutcome`] requiring complete layout recomputation.
    const fn recompute(control: KeyControl) -> Self {
        Self {
            control,
            layout: LayoutMode::Recompute,
        }
    }

    /// Creates an outcome that reuses retained layout.
    ///
    /// # Arguments
    ///
    /// * `control` — Key propagation control emitted by built-in handling.
    ///
    /// # Returns
    ///
    /// A [`KeyEventOutcome`] permitting retained-layout reuse.
    const fn reuse(control: KeyControl) -> Self {
        Self {
            control,
            layout: LayoutMode::Reuse,
        }
    }
}

/// Dispatches custom and built-in key behavior through a view tree.
pub(crate) fn handle_view_key_event<V>(view: &mut V, key: KeyEvent) -> Result<KeyControl>
where
    V: View + ?Sized,
{
    handle_view_key_event_with_layout(view, key).map(|outcome| outcome.control)
}

/// Dispatches one key and reports whether its redraw may reuse layout.
///
/// # Arguments
///
/// * `view` — View tree receiving the key.
/// * `key` — Key event to dispatch.
///
/// # Returns
///
/// A [`KeyEventOutcome`] containing propagation and layout requirements.
///
/// # Errors
///
/// Returns [`crate::Error`] if custom key handling fails.
fn handle_view_key_event_with_layout<V>(view: &mut V, key: KeyEvent) -> Result<KeyEventOutcome>
where
    V: View + ?Sized,
{
    let control = view.__dispatch_key_event(key)?;
    if control == KeyControl::Pass {
        return handle_default_view_key_event_with_layout(view, key);
    }

    Ok(KeyEventOutcome::recompute(control))
}

/// Handles built-in scrolling, focus, editing, and activation keys.
pub(crate) fn handle_default_view_key_event<V>(view: &mut V, key: KeyEvent) -> Result<KeyControl>
where
    V: View + ?Sized,
{
    handle_default_view_key_event_with_layout(view, key).map(|outcome| outcome.control)
}

/// Handles built-in key behavior and reports scroll-only repaint eligibility.
///
/// # Arguments
///
/// * `view` — View tree receiving built-in key behavior.
/// * `key` — Key event to handle.
///
/// # Returns
///
/// A [`KeyEventOutcome`] containing propagation and layout requirements.
///
/// # Errors
///
/// Returns [`crate::Error`] if activating a focused control fails.
fn handle_default_view_key_event_with_layout<V>(
    view: &mut V,
    key: KeyEvent,
) -> Result<KeyEventOutcome>
where
    V: View + ?Sized,
{
    if key.kind != KeyEventKind::Press {
        return Ok(KeyEventOutcome::recompute(KeyControl::Pass));
    }

    if let Some(control) = view.__handle_form_key(key) {
        clear_scroll_to_top_key_pending(view);
        return Ok(KeyEventOutcome::recompute(control));
    }

    if let Some(control) = view.__handle_focused_input_key(key) {
        clear_scroll_to_top_key_pending(view);
        return Ok(KeyEventOutcome::recompute(control));
    }

    let history_direction = match key.code {
        KeyCode::Char('H') => Some(true),
        KeyCode::Char('L') => Some(false),
        KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::SHIFT) => Some(true),
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::SHIFT) => Some(false),
        _ => None,
    };
    if let Some(back) = history_direction
        && view.__navigate_markdown_history(back)
    {
        clear_scroll_to_top_key_pending(view);
        return Ok(KeyEventOutcome::recompute(KeyControl::Handled));
    }

    let outcome = match key.code {
        KeyCode::Down | KeyCode::Char('j') => handle_scroll_key(view, 1),
        KeyCode::Up | KeyCode::Char('k') => handle_scroll_key(view, -1),
        KeyCode::PageDown => handle_scroll_key(view, 5),
        KeyCode::PageUp => handle_scroll_key(view, -5),
        KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => handle_scroll_key(view, 5),
        KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => handle_scroll_key(view, -5),
        KeyCode::Char('g') => handle_scroll_to_top_key(view),
        KeyCode::Char('G') => {
            clear_scroll_to_top_key_pending(view);
            scroll_key_outcome(view.__scroll_first_overflowing_to_bottom())
        }
        KeyCode::Tab | KeyCode::BackTab => {
            clear_scroll_to_top_key_pending(view);
            let count = view.__focusable_count();
            if count == 0 {
                KeyEventOutcome::recompute(KeyControl::Pass)
            } else {
                let direction = if key.code == KeyCode::Tab {
                    FocusDirection::Forward
                } else {
                    FocusDirection::Backward
                };
                move_focus(view, direction, count);
                KeyEventOutcome::recompute(KeyControl::Handled)
            }
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            clear_scroll_to_top_key_pending(view);
            KeyEventOutcome::recompute(
                view.__activate_focused_button()?
                    .map_or(KeyControl::Pass, KeyControl::from),
            )
        }
        _ => {
            clear_scroll_to_top_key_pending(view);
            KeyEventOutcome::recompute(KeyControl::Pass)
        }
    };

    Ok(outcome)
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
///
/// # Arguments
///
/// * `view` — View tree containing the scroll target.
/// * `delta` — Signed vertical terminal-cell delta.
///
/// # Returns
///
/// A [`KeyEventOutcome`] describing propagation and layout reuse.
fn handle_scroll_key<V>(view: &mut V, delta: i16) -> KeyEventOutcome
where
    V: View + ?Sized,
{
    clear_scroll_to_top_key_pending(view);
    scroll_key_outcome(view.__scroll_first_overflowing(Axes::new(0, delta)))
}

/// Converts a built-in scroll result into its next-frame layout requirement.
///
/// # Arguments
///
/// * `scrolled` — Whether an overflowing container changed offset.
///
/// # Returns
///
/// A [`KeyEventOutcome`] permitting reuse only after successful scrolling.
fn scroll_key_outcome(scrolled: bool) -> KeyEventOutcome {
    if scrolled {
        KeyEventOutcome::reuse(KeyControl::Handled)
    } else {
        KeyEventOutcome::recompute(KeyControl::Pass)
    }
}

/// Handles the two-key `gg` scroll-to-top sequence.
///
/// # Arguments
///
/// * `view` — View tree containing the scroll target.
///
/// # Returns
///
/// A [`KeyEventOutcome`] describing sequence handling and layout reuse.
fn handle_scroll_to_top_key<V>(view: &mut V) -> KeyEventOutcome
where
    V: View + ?Sized,
{
    if take_scroll_to_top_key_pending(view) {
        scroll_key_outcome(view.__scroll_first_overflowing_to_top())
    } else if view.__has_overflowing_scroll_target() {
        set_scroll_to_top_key_pending(view, true);
        KeyEventOutcome::reuse(KeyControl::Handled)
    } else {
        KeyEventOutcome::recompute(KeyControl::Pass)
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
