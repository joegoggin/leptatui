//! Normal-mode commands, history, yanking, and paste behavior.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::KeyControl;

use super::{
    insert::commit_input_value,
    model::{EditableAction, EditableControlKind},
    movement::*,
    state::{EditableState, VimMode},
    visual::{enter_visual_mode, handle_pending_normal_mode_key, replace_value_range},
};

/// Handles normal-mode movement, command sequences, and mutations.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives proposed next values.
/// * `editable_state` — Retained cursor, mode, and history state for the control.
/// * `key` — Key event to apply while the control is in normal mode.
/// * `kind` — Editable control variant receiving the key.
///
/// # Returns
///
/// An [`Option`] containing a [`KeyControl`] value when normal-mode behavior
/// handles the key.
pub(crate) fn handle_normal_mode_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
    kind: EditableControlKind,
) -> Option<KeyControl> {
    if key.code == KeyCode::Char('r') && key.modifiers == KeyModifiers::CONTROL {
        editable_state.set_normal_key_pending(None);
        return Some(handle_redo_input_key(value, on_input, editable_state));
    }

    if let Some(pending) = editable_state.take_normal_key_pending() {
        return Some(handle_pending_normal_mode_key(
            value,
            on_input,
            editable_state,
            key,
            kind,
            pending,
        ));
    }

    let plain_key = !key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);

    match key.code {
        KeyCode::Esc => Some(KeyControl::Handled),
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
            handle_normal_vertical_key(value, editable_state, kind, text_area_previous_line_cursor)
        }
        KeyCode::Down | KeyCode::Char('j') if plain_key => {
            handle_normal_vertical_key(value, editable_state, kind, text_area_next_line_cursor)
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
        KeyCode::Char('d') if plain_key => {
            editable_state.set_normal_key_pending(Some('d'));
            Some(KeyControl::Handled)
        }
        KeyCode::Char('y') if plain_key => {
            editable_state.set_normal_key_pending(Some('y'));
            Some(KeyControl::Handled)
        }
        KeyCode::Char('v') if plain_key => {
            enter_visual_mode(value, editable_state, VimMode::Visual, kind);
            Some(KeyControl::Handled)
        }
        KeyCode::Char('V') if plain_key => {
            enter_visual_mode(value, editable_state, VimMode::VisualLine, kind);
            Some(KeyControl::Handled)
        }
        KeyCode::Char('x') if plain_key => Some(handle_delete_normal_char_key(
            value,
            on_input,
            editable_state,
        )),
        KeyCode::Char('p') if plain_key => Some(handle_paste_input_key(
            value,
            on_input,
            editable_state,
            kind,
        )),
        KeyCode::Char('u') if plain_key => {
            Some(handle_undo_input_key(value, on_input, editable_state))
        }
        KeyCode::Char('o') if plain_key => Some(handle_open_line_key(
            value,
            on_input,
            editable_state,
            kind,
            OpenLinePosition::Below,
        )),
        KeyCode::Char('O') if plain_key => Some(handle_open_line_key(
            value,
            on_input,
            editable_state,
            kind,
            OpenLinePosition::Above,
        )),
        KeyCode::Char('i') if plain_key => {
            editable_state.set_mode(VimMode::Insert);
            editable_state.set_cursor(clamp_cursor(value, editable_state.cursor()));
            Some(KeyControl::Handled)
        }
        KeyCode::Char('a') if plain_key => {
            editable_state.set_mode(VimMode::Insert);
            editable_state.set_cursor(insert_after_normal_cursor(value, editable_state.cursor()));
            Some(KeyControl::Handled)
        }
        KeyCode::Char('I') if plain_key => {
            editable_state.set_mode(VimMode::Insert);
            editable_state.set_cursor(line_start(value, editable_state.cursor(), kind));
            Some(KeyControl::Handled)
        }
        KeyCode::Char('A') if plain_key => {
            editable_state.set_mode(VimMode::Insert);
            editable_state.set_cursor(insert_line_end(value, editable_state.cursor(), kind));
            Some(KeyControl::Handled)
        }
        KeyCode::Enter | KeyCode::Backspace | KeyCode::Delete => Some(KeyControl::Handled),
        KeyCode::Char(_) if plain_key => Some(KeyControl::Handled),
        _ => None,
    }
}

/// Handles vertical normal-mode movement inside editable controls.
///
/// Returns [`None`] when the cursor cannot move within the editable control so
/// parent containers can handle boundary scrolling.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `editable_state` — Retained cursor, mode, selection, and history state.
/// * `kind` — Editable control variant receiving the key.
/// * `move_text_area_cursor` — Movement function used for text-area cursor rows.
///
/// # Returns
///
/// An [`Option<KeyControl>`] indicating whether the editable control handled
/// the movement.
pub(crate) fn handle_normal_vertical_key(
    value: &str,
    editable_state: &mut EditableState,
    kind: EditableControlKind,
    move_text_area_cursor: fn(&str, usize) -> usize,
) -> Option<KeyControl> {
    let cursor = normal_cursor(value, editable_state.cursor());
    let next_cursor = match kind {
        EditableControlKind::Input => cursor,
        EditableControlKind::TextArea => normal_cursor(value, move_text_area_cursor(value, cursor)),
    };

    if next_cursor == cursor {
        return None;
    }

    editable_state.set_cursor(next_cursor);
    Some(KeyControl::Handled)
}

/// Handles normal-mode `x`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives the shortened value.
/// * `editable_state` — Retained cursor, mode, and history state for the control.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that character deletion was handled.
pub(crate) fn handle_delete_normal_char_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
) -> KeyControl {
    if value.is_empty() {
        editable_state.set_cursor(0);
        return KeyControl::Handled;
    }

    let cursor = normal_cursor(value, editable_state.cursor());
    let next_boundary = next_char_boundary(value, cursor);
    let next = replace_value_range(value, cursor..next_boundary, "");
    let next_cursor = normal_cursor_after_change(&next, cursor);

    commit_input_value(value, on_input, editable_state, next, next_cursor)
}

/// Handles normal-mode `dd`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives the shortened value.
/// * `editable_state` — Retained cursor, mode, and history state for the control.
/// * `kind` — Editable control variant receiving the command.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that line deletion was handled.
pub(crate) fn handle_delete_line_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    kind: EditableControlKind,
) -> KeyControl {
    if value.is_empty() {
        editable_state.set_yank_buffer("");
        editable_state.set_cursor(0);
        return KeyControl::Handled;
    }

    match kind {
        EditableControlKind::Input => {
            editable_state.set_yank_buffer(value);
            commit_input_value(value, on_input, editable_state, String::new(), 0)
        }
        EditableControlKind::TextArea => {
            let content_range = text_area_line_content_range(value, editable_state.cursor());
            let deleted_line = value[content_range].to_owned();
            editable_state.set_linewise_yank_buffer(deleted_line);

            let delete_range = text_area_line_delete_range(value, editable_state.cursor());
            let delete_start = delete_range.start;
            let next = replace_value_range(value, delete_range, "");
            let next_cursor = normal_cursor_after_change(&next, delete_start);

            commit_input_value(value, on_input, editable_state, next, next_cursor)
        }
    }
}

/// Handles normal-mode `yy`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `editable_state` — Retained cursor, mode, and yank-buffer state for the
///   control.
/// * `kind` — Editable control variant receiving the command.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that line yanking was handled.
pub(crate) fn handle_yank_line_key(
    value: &str,
    editable_state: &mut EditableState,
    kind: EditableControlKind,
) -> KeyControl {
    match kind {
        EditableControlKind::Input => editable_state.set_yank_buffer(value),
        EditableControlKind::TextArea => {
            let range = text_area_line_content_range(value, editable_state.cursor());
            editable_state.set_linewise_yank_buffer(value[range].to_owned());
        }
    }

    KeyControl::Handled
}

/// Normal-mode line placement for Vim open-line commands.
#[derive(Clone, Copy)]
pub(crate) enum OpenLinePosition {
    /// Insert a line above the current logical line.
    Above,
    /// Insert a line below the current logical line.
    Below,
}

/// Handles normal-mode `o` and `O`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives the opened-line value.
/// * `editable_state` — Retained cursor, mode, and history state for the
///   control.
/// * `kind` — Editable control variant receiving the command.
/// * `position` — Whether to open the new line above or below the current line.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that the open-line command was handled.
pub(crate) fn handle_open_line_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    kind: EditableControlKind,
    position: OpenLinePosition,
) -> KeyControl {
    if kind == EditableControlKind::Input {
        return KeyControl::Handled;
    }

    editable_state.set_mode(VimMode::Insert);
    editable_state.set_normal_key_pending(None);

    if value.is_empty() {
        editable_state.set_cursor(0);
        return KeyControl::Handled;
    }

    let cursor = normal_cursor(value, editable_state.cursor());
    let insert_at = match position {
        OpenLinePosition::Above => text_area_line_start(value, cursor),
        OpenLinePosition::Below => text_area_line_end(value, cursor),
    };
    let next_cursor = match position {
        OpenLinePosition::Above => insert_at,
        OpenLinePosition::Below => insert_at.saturating_add(1),
    };

    let next = replace_value_range(value, insert_at..insert_at, "\n");

    commit_input_value(value, on_input, editable_state, next, next_cursor)
}

/// Handles normal-mode `p`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives the pasted value.
/// * `editable_state` — Retained cursor, mode, and yank-buffer state for the
///   control.
/// * `kind` — Editable control variant receiving the command.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that paste was handled.
pub(crate) fn handle_paste_input_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    kind: EditableControlKind,
) -> KeyControl {
    let yank_buffer = editable_state.yank_buffer().to_owned();
    if yank_buffer.is_empty() {
        return KeyControl::Handled;
    }

    let (next, next_cursor) =
        if kind == EditableControlKind::TextArea && editable_state.yank_linewise() {
            text_area_linewise_paste(value, editable_state.cursor(), &yank_buffer)
        } else {
            charwise_paste(value, editable_state.cursor(), &yank_buffer)
        };

    commit_input_value(value, on_input, editable_state, next, next_cursor)
}

/// Handles normal-mode `u`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives the restored value.
/// * `editable_state` — Retained cursor, mode, and undo-history state for the
///   control.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that undo was handled.
pub(crate) fn handle_undo_input_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
) -> KeyControl {
    let Some(on_input) = on_input.as_ref() else {
        return KeyControl::Handled;
    };
    let Some(previous) = editable_state.pop_undo() else {
        return KeyControl::Handled;
    };

    editable_state.push_redo(value.to_owned());
    let next_cursor =
        cursor_after_value_replace(&previous, editable_state.cursor(), editable_state.mode());
    editable_state.set_cursor(next_cursor);
    on_input(previous).into()
}

/// Handles normal-mode `Ctrl+r`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives the redone value.
/// * `editable_state` — Retained cursor, mode, and redo-history state for the
///   control.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that redo was handled.
pub(crate) fn handle_redo_input_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
) -> KeyControl {
    let Some(on_input) = on_input.as_ref() else {
        return KeyControl::Handled;
    };
    let Some(next) = editable_state.pop_redo() else {
        return KeyControl::Handled;
    };

    editable_state.push_undo(value.to_owned());
    let next_cursor =
        cursor_after_value_replace(&next, editable_state.cursor(), editable_state.mode());
    editable_state.set_cursor(next_cursor);
    on_input(next).into()
}
