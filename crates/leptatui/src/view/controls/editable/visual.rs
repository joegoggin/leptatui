//! Visual-mode selection and mutation behavior.

use std::ops::Range;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::KeyControl;

use super::{
    insert::commit_input_value,
    model::{EditableAction, EditableControlKind},
    movement::*,
    normal::{handle_delete_line_key, handle_yank_line_key},
    state::{EditableState, VimMode},
};

/// Handles visual-mode movement and selection mutations.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives proposed next values.
/// * `editable_state` — Retained cursor, mode, selection, and history state.
/// * `key` — Key event to apply while the control is in visual mode.
/// * `kind` — Editable control variant receiving the key.
///
/// # Returns
///
/// An [`Option`] containing a [`KeyControl`] value when visual-mode behavior
/// handles the key.
pub(crate) fn handle_visual_mode_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
    kind: EditableControlKind,
) -> Option<KeyControl> {
    if let Some(pending) = editable_state.take_normal_key_pending() {
        return Some(handle_pending_visual_mode_key(
            value,
            editable_state,
            key,
            pending,
        ));
    }

    let plain_key = !key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);

    match key.code {
        KeyCode::Esc => {
            exit_visual_mode(value, editable_state);
            Some(KeyControl::Handled)
        }
        KeyCode::Left | KeyCode::Char('h') if plain_key => {
            let cursor = normal_previous_char_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Right | KeyCode::Char('l') if plain_key => {
            let cursor = normal_next_char_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Up | KeyCode::Char('k') if plain_key => {
            let cursor = match kind {
                EditableControlKind::Input => normal_cursor(value, editable_state.cursor()),
                EditableControlKind::TextArea => normal_cursor(
                    value,
                    text_area_previous_line_cursor(value, editable_state.cursor()),
                ),
            };
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Down | KeyCode::Char('j') if plain_key => {
            let cursor = match kind {
                EditableControlKind::Input => normal_cursor(value, editable_state.cursor()),
                EditableControlKind::TextArea => normal_cursor(
                    value,
                    text_area_next_line_cursor(value, editable_state.cursor()),
                ),
            };
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Home | KeyCode::Char('0') if plain_key => {
            let cursor = line_start(value, editable_state.cursor(), kind);
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::End | KeyCode::Char('$') if plain_key => {
            let cursor = normal_line_end(value, editable_state.cursor(), kind);
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Char('w') if plain_key => {
            let cursor = next_word_start_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Char('b') if plain_key => {
            let cursor = previous_word_start_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Char('e') if plain_key => {
            let cursor = word_end_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Char('g') if plain_key => {
            editable_state.set_normal_key_pending(Some('g'));
            Some(KeyControl::Handled)
        }
        KeyCode::Char('G') if plain_key => {
            editable_state.set_cursor(normal_last_char_cursor(value));
            Some(KeyControl::Handled)
        }
        KeyCode::Char('v') if plain_key => {
            if editable_state.mode() == VimMode::Visual {
                exit_visual_mode(value, editable_state);
            } else {
                editable_state.set_mode(VimMode::Visual);
                ensure_visual_anchor(value, editable_state);
            }
            Some(KeyControl::Handled)
        }
        KeyCode::Char('V') if plain_key => {
            if editable_state.mode() == VimMode::VisualLine {
                exit_visual_mode(value, editable_state);
            } else {
                editable_state.set_mode(VimMode::VisualLine);
                ensure_visual_anchor(value, editable_state);
            }
            Some(KeyControl::Handled)
        }
        KeyCode::Char('y') if plain_key => Some(handle_yank_visual_selection_key(
            value,
            editable_state,
            kind,
        )),
        KeyCode::Char('d') | KeyCode::Char('x') if plain_key => Some(
            handle_delete_visual_selection_key(value, on_input, editable_state, kind),
        ),
        KeyCode::Enter | KeyCode::Backspace | KeyCode::Delete => Some(KeyControl::Handled),
        KeyCode::Char(_) if plain_key => Some(KeyControl::Handled),
        _ => None,
    }
}

/// Handles the second key in a visual-mode command sequence.
pub(crate) fn handle_pending_visual_mode_key(
    value: &str,
    editable_state: &mut EditableState,
    key: &KeyEvent,
    pending: char,
) -> KeyControl {
    let plain_key = !key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);

    match (pending, key.code) {
        ('g', KeyCode::Char('g')) if plain_key => {
            editable_state.set_cursor(0);
            KeyControl::Handled
        }
        _ => {
            ensure_visual_anchor(value, editable_state);
            KeyControl::Handled
        }
    }
}

/// Handles the second key in a normal-mode command sequence.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives proposed next values.
/// * `editable_state` — Retained cursor, mode, and history state for the control.
/// * `key` — Key event completing or cancelling the sequence.
/// * `kind` — Editable control variant receiving the key.
/// * `pending` — First key already captured for the sequence.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that the pending sequence was handled.
pub(crate) fn handle_pending_normal_mode_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
    kind: EditableControlKind,
    pending: char,
) -> KeyControl {
    let plain_key = !key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);

    match (pending, key.code) {
        ('g', KeyCode::Char('g')) if plain_key => {
            editable_state.set_cursor(0);
            KeyControl::Handled
        }
        ('d', KeyCode::Char('d')) if plain_key => {
            handle_delete_line_key(value, on_input, editable_state, kind)
        }
        ('y', KeyCode::Char('y')) if plain_key => handle_yank_line_key(value, editable_state, kind),
        _ => KeyControl::Handled,
    }
}

/// Enters a visual mode with the current normal cursor as the fixed anchor.
pub(crate) fn enter_visual_mode(
    value: &str,
    editable_state: &mut EditableState,
    mode: VimMode,
    _kind: EditableControlKind,
) {
    editable_state.set_normal_key_pending(None);
    let cursor = normal_cursor(value, editable_state.cursor());
    editable_state.set_cursor(cursor);
    editable_state.set_mode(mode);
    editable_state.set_selection_anchor(Some(cursor));
}

/// Leaves visual mode and clears selection state.
pub(crate) fn exit_visual_mode(value: &str, editable_state: &mut EditableState) {
    editable_state.set_normal_key_pending(None);
    editable_state.set_cursor(normal_cursor(value, editable_state.cursor()));
    editable_state.set_mode(VimMode::Normal);
}

/// Ensures a visual selection anchor exists after mode changes or stale state.
pub(crate) fn ensure_visual_anchor(value: &str, editable_state: &mut EditableState) {
    if editable_state.selection_anchor().is_none() {
        editable_state.set_selection_anchor(Some(normal_cursor(value, editable_state.cursor())));
    }
}

/// Returns the active visual selection range for rendering or mutation.
pub(crate) fn visual_selection_range(
    value: &str,
    editable_state: &EditableState,
    kind: EditableControlKind,
) -> Option<Range<usize>> {
    let anchor = editable_state.selection_anchor()?;
    let selection = match editable_state.mode() {
        VimMode::Visual => visual_charwise_range(value, anchor, editable_state.cursor()),
        VimMode::VisualLine => {
            visual_linewise_content_range(value, anchor, editable_state.cursor(), kind)
        }
        VimMode::Insert | VimMode::Normal => return None,
    };

    Some(selection)
}

/// Returns the inclusive character-wise visual selection as a byte range.
pub(crate) fn visual_charwise_range(value: &str, anchor: usize, cursor: usize) -> Range<usize> {
    if value.is_empty() {
        return 0..0;
    }

    let anchor = normal_cursor(value, anchor);
    let cursor = normal_cursor(value, cursor);
    if anchor <= cursor {
        anchor..next_char_boundary(value, cursor)
    } else {
        cursor..next_char_boundary(value, anchor)
    }
}

/// Returns the content bytes covered by a line-wise visual selection.
pub(crate) fn visual_linewise_content_range(
    value: &str,
    anchor: usize,
    cursor: usize,
    kind: EditableControlKind,
) -> Range<usize> {
    if value.is_empty() {
        return 0..0;
    }

    match kind {
        EditableControlKind::Input => 0..value.len(),
        EditableControlKind::TextArea => {
            let anchor = clamp_cursor(value, anchor);
            let cursor = clamp_cursor(value, cursor);
            let start =
                text_area_line_start(value, anchor).min(text_area_line_start(value, cursor));
            let end = text_area_line_end(value, anchor).max(text_area_line_end(value, cursor));

            start..end
        }
    }
}

/// Returns the bytes removed by a line-wise visual delete.
pub(crate) fn visual_linewise_delete_range(
    value: &str,
    content_range: Range<usize>,
) -> Range<usize> {
    if value.is_empty() {
        return 0..0;
    }

    if content_range.end < value.len() {
        content_range.start..content_range.end + 1
    } else if content_range.start > 0 {
        content_range.start - 1..content_range.end
    } else {
        content_range
    }
}

/// Handles visual-mode `y`.
pub(crate) fn handle_yank_visual_selection_key(
    value: &str,
    editable_state: &mut EditableState,
    kind: EditableControlKind,
) -> KeyControl {
    let selection = visual_selection_range(value, editable_state, kind).unwrap_or(0..0);
    if editable_state.mode() == VimMode::VisualLine && kind == EditableControlKind::TextArea {
        editable_state.set_linewise_yank_buffer(value[selection.clone()].to_owned());
    } else {
        editable_state.set_yank_buffer(value[selection.clone()].to_owned());
    }

    editable_state.set_cursor(normal_cursor_after_change(value, selection.start));
    editable_state.set_normal_key_pending(None);
    editable_state.set_mode(VimMode::Normal);
    KeyControl::Handled
}

/// Returns a copy of `value` with `range` replaced by `replacement`.
pub(crate) fn replace_value_range(value: &str, range: Range<usize>, replacement: &str) -> String {
    let mut next = String::with_capacity(
        value
            .len()
            .saturating_sub(range.len())
            .saturating_add(replacement.len()),
    );
    next.push_str(&value[..range.start]);
    next.push_str(replacement);
    next.push_str(&value[range.end..]);
    next
}

/// Handles visual-mode `d` and `x`.
pub(crate) fn handle_delete_visual_selection_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    kind: EditableControlKind,
) -> KeyControl {
    let selection = visual_selection_range(value, editable_state, kind).unwrap_or(0..0);
    let linewise =
        editable_state.mode() == VimMode::VisualLine && kind == EditableControlKind::TextArea;
    if linewise {
        editable_state.set_linewise_yank_buffer(value[selection.clone()].to_owned());
    } else {
        editable_state.set_yank_buffer(value[selection.clone()].to_owned());
    }

    let delete_range = if linewise {
        visual_linewise_delete_range(value, selection)
    } else {
        selection
    };
    let delete_start = delete_range.start;
    let next = replace_value_range(value, delete_range, "");
    let next_cursor = normal_cursor_after_change(&next, delete_start);

    editable_state.set_normal_key_pending(None);
    editable_state.set_mode(VimMode::Normal);
    commit_input_value(value, on_input, editable_state, next, next_cursor)
}
