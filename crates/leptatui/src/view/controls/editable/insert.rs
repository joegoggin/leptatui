//! Insert-mode dispatch and editable value commits.

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{AppControl, KeyControl};

use super::{
    model::{EditableAction, EditableControlKind},
    movement::*,
    normal::handle_normal_mode_key,
    state::{EditableState, PendingInsertKey, VimMode},
    visual::{handle_visual_mode_key, replace_value_range},
};

/// Maximum time allowed between insert-mode `j` and `k` escape keys.
const INSERT_ESCAPE_TIMEOUT: Duration = Duration::from_millis(1000);

/// Handles a focused input key and returns whether default propagation stops.
///
/// # Arguments
///
/// * `value` — Current controlled input value.
/// * `on_input` — Optional callback that receives proposed next values.
/// * `editable_state` — Retained cursor and scroll state for the input.
/// * `key` — Key event to apply to the input.
///
/// # Returns
///
/// An [`Option`] containing a [`KeyControl`] value when the key is handled by
/// input editing behavior.
pub(crate) fn handle_input_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
) -> Option<KeyControl> {
    handle_editable_key(
        value,
        on_input,
        editable_state,
        key,
        EditableControlKind::Input,
    )
}

/// Handles a focused text-area key and returns whether default propagation stops.
///
/// # Arguments
///
/// * `value` — Current controlled text-area value.
/// * `on_input` — Optional callback that receives proposed next values.
/// * `editable_state` — Retained cursor and scroll state for the text area.
/// * `key` — Key event to apply to the text area.
///
/// # Returns
///
/// An [`Option`] containing a [`KeyControl`] value when the key is handled by
/// text-area editing behavior.
pub(crate) fn handle_text_area_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
) -> Option<KeyControl> {
    handle_editable_key(
        value,
        on_input,
        editable_state,
        key,
        EditableControlKind::TextArea,
    )
}

/// Handles a focused editable-control key according to its Vim mode.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives proposed next values.
/// * `editable_state` — Retained cursor, mode, and history state for the control.
/// * `key` — Key event to apply to the control.
/// * `kind` — Editable control variant receiving the key.
///
/// # Returns
///
/// An [`Option`] containing a [`KeyControl`] value when editable behavior
/// handles the key.
pub(crate) fn handle_editable_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
    kind: EditableControlKind,
) -> Option<KeyControl> {
    match editable_state.mode() {
        VimMode::Insert => handle_insert_mode_key(value, on_input, editable_state, key, kind),
        VimMode::Normal => handle_normal_mode_key(value, on_input, editable_state, key, kind),
        VimMode::Visual | VimMode::VisualLine => {
            handle_visual_mode_key(value, on_input, editable_state, key, kind)
        }
    }
}

/// Handles insert-mode editing and cursor movement for a focused control.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `on_input` — Optional callback that receives proposed next values.
/// * `editable_state` — Retained cursor, mode, and history state for the control.
/// * `key` — Key event to apply while the control is in insert mode.
/// * `kind` — Editable control variant receiving the key.
///
/// # Returns
///
/// An [`Option`] containing a [`KeyControl`] value when insert-mode behavior
/// handles the key.
pub(crate) fn handle_insert_mode_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
    kind: EditableControlKind,
) -> Option<KeyControl> {
    editable_state.set_normal_key_pending(None);

    let plain_key = !key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
    let now = Instant::now();

    if let Some(pending) = editable_state.take_insert_key_pending() {
        return Some(handle_pending_insert_mode_key(
            value,
            on_input,
            editable_state,
            key,
            kind,
            pending,
            now,
        ));
    }

    match key.code {
        KeyCode::Esc => {
            exit_insert_mode(value, editable_state);
            Some(KeyControl::Handled)
        }
        KeyCode::Left => {
            let cursor = previous_char_boundary(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Right => {
            let cursor = next_char_boundary(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Home => {
            let cursor = match kind {
                EditableControlKind::Input => 0,
                EditableControlKind::TextArea => {
                    text_area_line_start(value, editable_state.cursor())
                }
            };
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::End => {
            let cursor = insert_line_end(value, editable_state.cursor(), kind);
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Up if kind == EditableControlKind::TextArea => {
            let cursor = text_area_previous_line_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Down if kind == EditableControlKind::TextArea => {
            let cursor = text_area_next_line_cursor(value, editable_state.cursor());
            editable_state.set_cursor(cursor);
            Some(KeyControl::Handled)
        }
        KeyCode::Enter if kind == EditableControlKind::TextArea => Some(handle_insert_input_key(
            value,
            on_input,
            editable_state,
            '\n',
        )),
        KeyCode::Enter => Some(KeyControl::Handled),
        KeyCode::Backspace => Some(handle_backspace_input_key(
            value,
            on_input,
            editable_state,
            kind,
        )),
        KeyCode::Delete => Some(handle_delete_input_key(value, on_input, editable_state)),
        KeyCode::Char('j') if plain_key => {
            editable_state.set_insert_key_pending('j', now);
            Some(KeyControl::Handled)
        }
        KeyCode::Char(character) if plain_key => Some(handle_insert_input_key(
            value,
            on_input,
            editable_state,
            character,
        )),
        _ => None,
    }
}

/// Handles the second key in an insert-mode key sequence.
pub(crate) fn handle_pending_insert_mode_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
    kind: EditableControlKind,
    pending: PendingInsertKey,
    now: Instant,
) -> KeyControl {
    let plain_key = !key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);

    if insert_key_pending_expired(pending, now) {
        return handle_expired_pending_insert_mode_key(
            value,
            on_input,
            editable_state,
            key,
            kind,
            pending.key(),
            plain_key,
        );
    }

    let pending_key = pending.key();

    match (pending_key, key.code) {
        ('j', KeyCode::Char('k')) if plain_key => {
            exit_insert_mode(value, editable_state);
            KeyControl::Handled
        }
        ('j', KeyCode::Char(character)) if plain_key => {
            let mut inserted = String::with_capacity(pending_key.len_utf8() + character.len_utf8());
            inserted.push(pending_key);
            inserted.push(character);
            handle_insert_text_key(value, on_input, editable_state, &inserted)
        }
        ('j', KeyCode::Esc) => {
            exit_insert_mode(value, editable_state);
            KeyControl::Handled
        }
        ('j', KeyCode::Backspace) => {
            editable_state.set_cursor(clamp_cursor(value, editable_state.cursor()));
            KeyControl::Handled
        }
        ('j', KeyCode::Enter) if plain_key && kind == EditableControlKind::TextArea => {
            let mut text = String::with_capacity(pending_key.len_utf8() + 1);
            text.push(pending_key);
            text.push('\n');
            handle_insert_text_key(value, on_input, editable_state, &text)
        }
        ('j', KeyCode::Enter) if plain_key => {
            handle_insert_input_key(value, on_input, editable_state, pending_key)
        }
        ('j', _) => handle_insert_input_key(value, on_input, editable_state, pending_key),
        _ => KeyControl::Handled,
    }
}

/// Handles a key received after an insert-mode sequence times out.
pub(crate) fn handle_expired_pending_insert_mode_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    key: &KeyEvent,
    kind: EditableControlKind,
    pending: char,
    plain_key: bool,
) -> KeyControl {
    match key.code {
        KeyCode::Char(character) if plain_key => {
            let mut inserted = String::with_capacity(pending.len_utf8() + character.len_utf8());
            inserted.push(pending);
            inserted.push(character);
            handle_insert_text_key(value, on_input, editable_state, &inserted)
        }
        KeyCode::Enter if plain_key && kind == EditableControlKind::TextArea => {
            let mut text = String::with_capacity(pending.len_utf8() + 1);
            text.push(pending);
            text.push('\n');
            handle_insert_text_key(value, on_input, editable_state, &text)
        }
        _ => handle_insert_input_key(value, on_input, editable_state, pending),
    }
}

/// Returns whether a pending insert-mode key sequence has timed out.
pub(crate) fn insert_key_pending_expired(pending: PendingInsertKey, now: Instant) -> bool {
    now.saturating_duration_since(pending.started_at()) >= INSERT_ESCAPE_TIMEOUT
}

/// Returns pending insert-mode key state that is still within the timeout.
pub(crate) fn active_insert_key_pending(
    editable_state: &EditableState,
    now: Instant,
) -> Option<PendingInsertKey> {
    let pending = editable_state.insert_key_pending()?;
    (!insert_key_pending_expired(pending, now)).then_some(pending)
}

/// Returns whether the editable control has an unexpired pending insert key.
pub(crate) fn has_active_insert_key_pending(editable_state: &EditableState, now: Instant) -> bool {
    active_insert_key_pending(editable_state, now).is_some()
}

/// Emits an expired pending insert-mode key, if one exists.
pub(crate) fn flush_expired_insert_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    now: Instant,
) -> Option<AppControl> {
    let pending = editable_state.insert_key_pending()?;
    if !insert_key_pending_expired(pending, now) {
        return None;
    }

    let pending = editable_state.take_insert_key_pending()?;
    Some(handle_insert_input_key(value, on_input, editable_state, pending.key()).into())
}

/// Leaves insert mode using the same cursor placement as Esc.
pub(crate) fn exit_insert_mode(value: &str, editable_state: &mut EditableState) {
    editable_state.set_mode(VimMode::Normal);
    editable_state.set_cursor(normal_cursor_from_insert(value, editable_state.cursor()));
}

/// Handles insertion for a focused editable text control.
///
/// # Arguments
///
/// * `value` — Current controlled value.
/// * `on_input` — Optional callback that receives the inserted value.
/// * `editable_state` — Retained cursor and scroll state for the control.
/// * `character` — Character to insert at the cursor.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that insertion was handled.
pub(crate) fn handle_insert_input_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    character: char,
) -> KeyControl {
    let mut inserted = String::with_capacity(character.len_utf8());
    inserted.push(character);
    handle_insert_text_key(value, on_input, editable_state, &inserted)
}

/// Handles text insertion for a focused editable text control.
///
/// # Arguments
///
/// * `value` — Current controlled value.
/// * `on_input` — Optional callback that receives the inserted value.
/// * `editable_state` — Retained cursor and scroll state for the control.
/// * `inserted` — Text to insert at the cursor.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that insertion was handled.
pub(crate) fn handle_insert_text_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    inserted: &str,
) -> KeyControl {
    let cursor = clamp_cursor(value, editable_state.cursor());
    let next = replace_value_range(value, cursor..cursor, inserted);

    commit_input_value(
        value,
        on_input,
        editable_state,
        next,
        cursor.saturating_add(inserted.len()),
    )
}

/// Handles backspace for a focused editable text control.
///
/// # Arguments
///
/// * `value` — Current controlled value.
/// * `on_input` — Optional callback that receives the shortened value.
/// * `editable_state` — Retained cursor and scroll state for the control.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that backspace was handled.
pub(crate) fn handle_backspace_input_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    kind: EditableControlKind,
) -> KeyControl {
    let cursor = clamp_cursor(value, editable_state.cursor());
    if cursor == 0 {
        if kind == EditableControlKind::TextArea
            && let Some(next) = value.strip_prefix('\n')
        {
            return commit_input_value(value, on_input, editable_state, next.to_owned(), 0);
        }
        editable_state.set_cursor(0);
        return KeyControl::Handled;
    }

    let previous = previous_char_boundary(value, cursor);
    let next = replace_value_range(value, previous..cursor, "");

    commit_input_value(value, on_input, editable_state, next, previous)
}

/// Handles delete for a focused editable text control.
///
/// # Arguments
///
/// * `value` — Current controlled value.
/// * `on_input` — Optional callback that receives the shortened value.
/// * `editable_state` — Retained cursor and scroll state for the control.
///
/// # Returns
///
/// A [`KeyControl`] value indicating that delete was handled.
pub(crate) fn handle_delete_input_key(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
) -> KeyControl {
    let cursor = clamp_cursor(value, editable_state.cursor());
    if cursor == value.len() {
        editable_state.set_cursor(cursor);
        return KeyControl::Handled;
    }

    let next_boundary = next_char_boundary(value, cursor);
    let next = replace_value_range(value, cursor..next_boundary, "");

    commit_input_value(value, on_input, editable_state, next, cursor)
}

/// Emits a controlled editable value update when a callback exists.
///
/// # Arguments
///
/// * `value` — Current controlled value before the proposed update.
/// * `on_input` — Optional callback that receives the proposed value.
/// * `editable_state` — Retained cursor and scroll state for the control.
/// * `next` — Proposed next controlled value.
/// * `next_cursor` — Cursor byte index to retain after emitting the value.
///
/// # Returns
///
/// A [`KeyControl`] value produced by the callback or handled by default when
/// no callback exists.
pub(crate) fn commit_input_value(
    value: &str,
    on_input: &Option<EditableAction>,
    editable_state: &mut EditableState,
    next: String,
    next_cursor: usize,
) -> KeyControl {
    let Some(on_input) = on_input.as_ref() else {
        return KeyControl::Handled;
    };

    if next != value {
        editable_state.push_undo(value.to_owned());
        editable_state.clear_redo();
    }
    editable_state.set_cursor(next_cursor);
    on_input(next).into()
}
